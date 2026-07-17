import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:crypto/crypto.dart' as crypto;
import 'package:path/path.dart' as p;

import '../core/mod_ffi.dart';
import 'managed_project_lock.dart';
import 'project_atomic_io.dart';
import 'revision3_content_index.dart';
import 'revision3_dialog_localization_authoring.dart';
import 'revision3_dialog_line_authoring.dart';
import 'revision3_npc_greeting_authoring.dart';
import 'revision3_project_history.dart';
import 'revision3_quest_transcript_authoring.dart';
import 'revision3_voice_take_preview_authoring.dart';

const int _maxManagedHeadBytes = 64 * 1024;

class ManagedProjectSessionException implements Exception {
  const ManagedProjectSessionException(this.message);

  final String message;

  @override
  String toString() => 'ManagedProjectSessionException: $message';
}

class ManagedProjectAlreadyInitializedException
    extends ManagedProjectSessionException {
  const ManagedProjectAlreadyInitializedException(super.message);

  @override
  String toString() => 'ManagedProjectAlreadyInitializedException: $message';
}

class ManagedProjectHeadConflictException
    extends ManagedProjectSessionException {
  const ManagedProjectHeadConflictException(super.message);

  @override
  String toString() => 'ManagedProjectHeadConflictException: $message';
}

class ManagedProjectVerificationException
    extends ManagedProjectSessionException {
  const ManagedProjectVerificationException(super.message);

  @override
  String toString() => 'ManagedProjectVerificationException: $message';
}

/// Native proved that exact-snapshot export failed before its no-clobber
/// publication boundary, but the failure also invalidated this session's Store
/// authority. The chosen output is known to be absent while the project still
/// requires a verified reopen before further authoring.
final class ManagedRevision3ExactSnapshotExportPrepublicationException
    extends ManagedProjectVerificationException {
  const ManagedRevision3ExactSnapshotExportPrepublicationException({
    required this.code,
    required String message,
  }) : super(message);

  final String code;

  @override
  String toString() =>
      'ManagedRevision3ExactSnapshotExportPrepublicationException($code): $message';
}

class ManagedProjectSessionClosedException
    extends ManagedProjectSessionException {
  const ManagedProjectSessionClosedException(super.message);

  @override
  String toString() => 'ManagedProjectSessionClosedException: $message';
}

class ManagedProjectReentrantOperationException
    extends ManagedProjectSessionException {
  const ManagedProjectReentrantOperationException(super.message);

  @override
  String toString() => 'ManagedProjectReentrantOperationException: $message';
}

/// The selected Quest/NPC or its generated ScriptModule no longer matches the
/// exact project generation that owns this managed session.
///
/// This is a caller-visible selection race, not Store uncertainty. It is safe
/// to refresh the content index and retry while the published head stays exact.
final class ManagedRevision3CompilerSelectionStaleException
    extends ManagedProjectSessionException {
  const ManagedRevision3CompilerSelectionStaleException()
    : super('the selected compiler target is stale for the exact project');
}

/// The editor plan no longer names the exact entity generation owned by this
/// still-healthy session. Refreshing the content index/seed is sufficient.
final class ManagedRevision3DialogLocalizationEditStaleException
    extends ManagedProjectSessionException {
  const ManagedRevision3DialogLocalizationEditStaleException()
    : super('the selected dialog localization is stale for the exact project');
}

/// The reviewed Voice-folder plan no longer names the exact source/project
/// generation owned by this session. The published project remains untouched;
/// callers may request a fresh plan while the session itself is still exact.
final class Revision3VoiceBatchStaleCheckpointException
    extends ManagedProjectSessionException {
  const Revision3VoiceBatchStaleCheckpointException()
    : super('the reviewed Voice folder plan is stale for the exact project');
}

/// A higher layer observed an uncertain Voice-folder publication receipt and
/// deliberately revoked this session's authoring authority. Only verified
/// recovery or closing and reopening the project can restore it.
final class Revision3VoiceBatchRequiresReopenException
    extends ManagedProjectSessionException {
  const Revision3VoiceBatchRequiresReopenException()
    : super('the Voice folder import requires the project to be reopened');
}

/// Evidence from one compiler-only check plus the app-side post-call Store
/// audit. No compiled artifact is retained and this receipt grants no build,
/// runtime, deployment, or publication authority.
final class ManagedRevision3CompilerCheckReceipt {
  const ManagedRevision3CompilerCheckReceipt({
    required this.result,
    required this.storeStillExactCurrent,
  });

  final AuthoringRevision3ManagedCompilerCheckResult result;

  /// False when the fixed Store head changed after native produced [result].
  /// The evidence and any recovery instructions remain reportable, but another
  /// managed operation requires verified recovery or reopening the project.
  final bool storeStillExactCurrent;

  bool get exactCurrent => storeStillExactCurrent && result.exactCurrent;

  bool get acceptedAtExactCurrent =>
      storeStillExactCurrent && result.acceptedAtExactCurrent;

  bool get recoveryRequired => result.recoveryRequired;
}

/// One fully verified in-session recovery of an uncertain revision-3 head.
///
/// Recovery only reconciles the crash-safe project head while this session
/// keeps its exclusive project lock. It grants no game, save-game, build,
/// deployment, or runtime authority.
final class ManagedRevision3RecoveryCheckpoint {
  const ManagedRevision3RecoveryCheckpoint({
    required this.previousHead,
    required this.recoveredHead,
    required this.projectId,
    required this.previousProjectRevision,
    required this.recoveredProjectRevision,
    required this.repairOutcome,
    required this.canonicalProjectJson,
  });

  final AuthoringWorkingHead previousHead;
  final AuthoringWorkingHead recoveredHead;
  final String projectId;
  final int previousProjectRevision;
  final int recoveredProjectRevision;
  final AtomicRepairOutcome repairOutcome;

  /// Exact canonical project bytes returned by the full recovered Store open.
  final String canonicalProjectJson;

  bool get advanced => recoveredProjectRevision == previousProjectRevision + 1;
}

/// One history restore returned only after full candidate reopen, exact
/// fixed-head publication, and full published reopen.
///
/// [restoredFromHead] remains an older immutable checkpoint. The published
/// [head] is always a fresh current+1 descendant of [previousHead].
final class ManagedRevision3ProjectHistoryRestoreCheckpoint {
  const ManagedRevision3ProjectHistoryRestoreCheckpoint({
    required this.previousHead,
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.previousProjectRevision,
    required this.projectRevision,
    required this.restoredFromHead,
    required this.restoredFromRevision,
  });

  final AuthoringWorkingHead previousHead;
  final AuthoringWorkingHead head;
  final String projectJson;
  final String projectId;
  final int previousProjectRevision;
  final int projectRevision;
  final AuthoringWorkingHead restoredFromHead;
  final int restoredFromRevision;
}

/// One immutable decision produced from the exact latest project inside a managed session's
/// serialized operation lane.
sealed class ManagedProjectDerivedSave<T> {
  const ManagedProjectDerivedSave();

  T get value;
}

/// Publish [projectJson] through the managed store before returning [value] to the caller.
final class ManagedProjectDerivedCandidate<T>
    extends ManagedProjectDerivedSave<T> {
  const ManagedProjectDerivedCandidate({
    required this.projectJson,
    required this.value,
  });

  final String projectJson;
  @override
  final T value;
}

/// Return [value] without preparing objects or touching the published head.
final class ManagedProjectDerivedRejection<T>
    extends ManagedProjectDerivedSave<T> {
  const ManagedProjectDerivedRejection(this.value);

  @override
  final T value;
}

typedef ManagedProjectDeriver<T> =
    FutureOr<ManagedProjectDerivedSave<T>> Function(String latestProjectJson);

/// One structurally verified revision-3 Quest checkpoint returned only after fixed-head CAS
/// publication and a full reopen. It deliberately carries no build, runtime, deployment, source,
/// or artifact-authority claim.
final class ManagedRevision3QuestDraftCheckpoint {
  const ManagedRevision3QuestDraftCheckpoint._({
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.projectRevision,
    required this.questId,
    required this.scriptModuleId,
    required this.artifactDeduplicated,
  });

  final AuthoringWorkingHead head;
  final String projectJson;
  final String projectId;
  final int projectRevision;
  final String questId;
  final String scriptModuleId;
  final bool artifactDeduplicated;
}

/// One existing Quest/module outline pair returned only after native
/// preparation, full candidate reopen, fixed-head CAS publication, and full
/// published reopen. Build remains blocked and runtime remains unqualified.
final class ManagedRevision3QuestOutlineEditCheckpoint {
  const ManagedRevision3QuestOutlineEditCheckpoint._({
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.projectRevision,
    required this.questId,
    required this.moduleId,
    required this.questRevision,
    required this.moduleRevision,
  });

  final AuthoringWorkingHead head;
  final String projectJson;
  final String projectId;
  final int projectRevision;
  final String questId;
  final String moduleId;
  final int questRevision;
  final int moduleRevision;
}

/// One exact Quest transcript returned only after native preparation, full
/// candidate reopen, guarded fixed-head CAS and a full published reopen.
final class ManagedRevision3QuestTranscriptCheckpoint {
  const ManagedRevision3QuestTranscriptCheckpoint._({
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.projectRevision,
    required this.questId,
    required this.questRevision,
    required this.moduleId,
    required this.moduleRevision,
    required this.mode,
    required this.transcriptCount,
    required this.createdLineId,
    required this.createdLocalizationId,
    required this.createdVoiceSlotId,
    required this.localizationAction,
  });

  final AuthoringWorkingHead head;
  final String projectJson;
  final String projectId;
  final int projectRevision;
  final String questId;
  final int questRevision;
  final String moduleId;
  final int moduleRevision;
  final AuthoringRevision3QuestTranscriptMode mode;
  final int transcriptCount;
  final String? createdLineId;
  final String? createdLocalizationId;
  final String? createdVoiceSlotId;
  final AuthoringRevision3DialogLocalizationAction? localizationAction;
}

/// One exact NPC greeting list returned only after native preparation, full
/// candidate reopen, guarded fixed-head CAS and a full published reopen.
final class ManagedRevision3NpcGreetingCheckpoint {
  const ManagedRevision3NpcGreetingCheckpoint._({
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.projectRevision,
    required this.npcId,
    required this.npcRevision,
    required this.moduleId,
    required this.moduleRevision,
    required this.mode,
    required this.greetingCount,
    required this.createdLineId,
    required this.createdLocalizationId,
    required this.createdVoiceSlotId,
    required this.localizationAction,
  });

  final AuthoringWorkingHead head;
  final String projectJson;
  final String projectId;
  final int projectRevision;
  final String npcId;
  final int npcRevision;
  final String moduleId;
  final int moduleRevision;
  final AuthoringRevision3NpcGreetingMode mode;
  final int greetingCount;
  final String? createdLineId;
  final String? createdLocalizationId;
  final String? createdVoiceSlotId;
  final AuthoringRevision3DialogLocalizationAction? localizationAction;
}

/// One existing Quest transition plan returned only after exact native
/// preparation, full candidate reopen, fixed-head CAS publication and a full
/// published reopen. It remains explicitly build-blocked, runtime-unqualified
/// and unsupported for native publication.
final class ManagedRevision3QuestTransitionsEditCheckpoint {
  const ManagedRevision3QuestTransitionsEditCheckpoint._({
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.projectRevision,
    required this.questId,
    required this.moduleId,
    required this.questRevision,
    required this.moduleRevision,
    required this.previousGeneratorVersion,
    required this.upgradedFromLegacy,
    required this.transitionPlanSeal,
    required this.buildStatus,
    required this.runtimeStatus,
    required this.publicationStatus,
  });

  final AuthoringWorkingHead head;
  final String projectJson;
  final String projectId;
  final int projectRevision;
  final String questId;
  final String moduleId;
  final int questRevision;
  final int moduleRevision;
  final int previousGeneratorVersion;
  final bool upgradedFromLegacy;
  final AuthoringDraftContentSeal transitionPlanSeal;
  final AuthoringRevision3QuestTransitionsBuildStatus buildStatus;
  final AuthoringRevision3QuestTransitionsRuntimeStatus runtimeStatus;
  final AuthoringRevision3QuestTransitionsPublicationStatus publicationStatus;
}

/// One existing Quest context/module pair returned only after fresh-catalog
/// native preparation, full candidate reopen, fixed-head CAS publication and
/// full published reopen.
final class ManagedRevision3QuestContextEditCheckpoint {
  const ManagedRevision3QuestContextEditCheckpoint._({
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.projectRevision,
    required this.questId,
    required this.moduleId,
    required this.questRevision,
    required this.moduleRevision,
    required this.parentRuntimeClass,
    required this.giverRuntimeUniqueName,
  });

  final AuthoringWorkingHead head;
  final String projectJson;
  final String projectId;
  final int projectRevision;
  final String questId;
  final String moduleId;
  final int questRevision;
  final int moduleRevision;
  final String parentRuntimeClass;
  final String giverRuntimeUniqueName;
}

/// One NPC Draft/module pair returned only after its native candidate was fully reopened,
/// fixed-head CAS published, and fully reopened again. It grants no build, runtime, catalog,
/// collision, source-inspection, spawn, deployment, or native-publication authority.
final class ManagedRevision3NpcDraftCheckpoint {
  const ManagedRevision3NpcDraftCheckpoint._({
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.projectRevision,
    required this.npcId,
    required this.scriptModuleId,
    required this.displayName,
    required this.moduleNamespace,
    required this.uniqueName,
    required this.parentCatalogId,
  });

  final AuthoringWorkingHead head;
  final String projectJson;
  final String projectId;
  final int projectRevision;
  final String npcId;
  final String scriptModuleId;
  final String displayName;
  final String moduleNamespace;
  final String uniqueName;
  final String parentCatalogId;
}

/// One existing NPC profile edit returned only after native preparation, full
/// candidate reopen, fixed-head CAS publication, and full published reopen.
final class ManagedRevision3NpcProfileEditCheckpoint {
  const ManagedRevision3NpcProfileEditCheckpoint._({
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.projectRevision,
    required this.npcId,
    required this.npcRevision,
    required this.scriptModuleId,
    required this.scriptModuleRevision,
    required this.displayName,
    required this.previousParentCatalogId,
    required this.parentCatalogId,
    required this.nameChanged,
    required this.archetypeChanged,
    required this.moduleRegenerated,
  });

  final AuthoringWorkingHead head;
  final String projectJson;
  final String projectId;
  final int projectRevision;
  final String npcId;
  final int npcRevision;
  final String scriptModuleId;
  final int scriptModuleRevision;
  final String displayName;
  final String previousParentCatalogId;
  final String parentCatalogId;
  final bool nameChanged;
  final bool archetypeChanged;
  final bool moduleRegenerated;
}

/// One project-local DialogLine prerequisite returned only after native
/// preparation, full candidate reopen, guarded fixed-head publication, and a
/// full published reopen. It grants no topic, build, runtime, game, or save
/// authority.
final class ManagedRevision3DialogLineEntryCheckpoint {
  const ManagedRevision3DialogLineEntryCheckpoint._({
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.projectRevision,
    required this.lineId,
    required this.localizationId,
    required this.localizationAction,
    required this.voiceSlotId,
  });

  final AuthoringWorkingHead head;
  final String projectJson;
  final String projectId;
  final int projectRevision;
  final String lineId;
  final String localizationId;
  final AuthoringRevision3DialogLocalizationAction localizationAction;
  final String? voiceSlotId;
}

/// One exact authored LocalizationEntry replacement returned only after native
/// preparation, full candidate reopen, fixed-head CAS publication, and a full
/// published reopen. It grants no topic, build, runtime, game, save, or native
/// publication authority.
final class ManagedRevision3DialogLocalizationEditCheckpoint {
  const ManagedRevision3DialogLocalizationEditCheckpoint._({
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.projectRevision,
    required this.localizationId,
    required this.localizationRevision,
    required this.addedLocales,
    required this.removedLocales,
  });

  final AuthoringWorkingHead head;
  final String projectJson;
  final String projectId;
  final int projectRevision;
  final String localizationId;
  final int localizationRevision;
  final List<String> addedLocales;
  final List<String> removedLocales;
}

/// One imported VoiceTake returned only after its native candidate was fully reopened,
/// fixed-head CAS published, and fully reopened again. A new slot starts unresolved; an existing
/// slot preserves its valid target evidence. This value itself grants no archive-member, build,
/// runtime, deployment, or native-publication claim.
final class ManagedRevision3VoiceTakeCheckpoint {
  const ManagedRevision3VoiceTakeCheckpoint._({
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.projectRevision,
    required this.lineId,
    required this.localizationId,
    required this.slotId,
    required this.takeId,
    required this.locale,
    required this.takeStatus,
    required this.slotCreated,
    required this.selected,
    required this.asset,
    required this.ogg,
    required this.assetDeduplicated,
  });

  final AuthoringWorkingHead head;
  final String projectJson;
  final String projectId;
  final int projectRevision;
  final String lineId;
  final String localizationId;
  final String slotId;
  final String takeId;
  final String locale;
  final AuthoringRevision3VoiceTakeStatus takeStatus;
  final bool slotCreated;
  final bool selected;
  final AuthoringRevision3VoiceAsset asset;
  final AuthoringRevision3VoiceOggMetadata ogg;
  final bool assetDeduplicated;
}

/// One all-or-nothing folder import returned only after every sealed source
/// produced one exact candidate, the single candidate project fully reopened,
/// fixed-head CAS publication succeeded, and the published head reopened.
/// It grants no build, deployment, runtime, game, or save authority.
final class ManagedRevision3VoiceBatchCheckpoint {
  ManagedRevision3VoiceBatchCheckpoint._({
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.projectRevision,
    required this.locale,
    required this.sourceManifestSha256,
    required this.planSha256,
    required this.importedCount,
    required this.alreadyPresentCount,
    required List<AuthoringRevision3VoiceBatchPreparationItem> items,
  }) : items = List.unmodifiable(items);

  final AuthoringWorkingHead head;
  final String projectJson;
  final String projectId;
  final int projectRevision;
  final String locale;
  final String sourceManifestSha256;
  final String planSha256;
  final int importedCount;
  final int alreadyPresentCount;
  final List<AuthoringRevision3VoiceBatchPreparationItem> items;
}

/// One changed or cleared VoiceSlot selection returned only after native
/// preparation, full candidate reopen, fixed-head CAS publication, and a full
/// published reopen. Build remains blocked and runtime remains unqualified.
final class ManagedRevision3VoiceTakeSelectionCheckpoint {
  const ManagedRevision3VoiceTakeSelectionCheckpoint._({
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.projectRevision,
    required this.lineId,
    required this.slotId,
    required this.slotRevision,
    required this.locale,
    required this.locId,
    required this.previousSelectedTakeId,
    required this.selectedTakeId,
  });

  final AuthoringWorkingHead head;
  final String projectJson;
  final String projectId;
  final int projectRevision;
  final String lineId;
  final String slotId;
  final int slotRevision;
  final String locale;
  final String locId;
  final String? previousSelectedTakeId;
  final String? selectedTakeId;
}

/// One Voice take detached from one exact line/language slot after native
/// preparation, full candidate reopen, fixed-head CAS publication, and a full
/// published reopen. Immutable audio CAS metadata remains preserved.
final class ManagedRevision3VoiceTakeRemovalCheckpoint {
  const ManagedRevision3VoiceTakeRemovalCheckpoint._({
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.projectRevision,
    required this.lineId,
    required this.localizationId,
    required this.slotId,
    required this.slotRevision,
    required this.locale,
    required this.locId,
    required this.takeId,
    required this.takeRevision,
    required this.previousSelectedTakeId,
    required this.selectionCleared,
    required this.takeEntityRemoved,
    required this.remainingCandidateCount,
  });

  final AuthoringWorkingHead head;
  final String projectJson;
  final String projectId;
  final int projectRevision;
  final String lineId;
  final String localizationId;
  final String slotId;
  final int slotRevision;
  final String locale;
  final String locId;
  final String takeId;
  final int takeRevision;
  final String? previousSelectedTakeId;
  final bool selectionCleared;
  final bool takeEntityRemoved;
  final int remainingCandidateCount;
}

/// One exact empty dialog VoiceSlot removed after native preparation, full
/// candidate reopen, fixed-head CAS publication, and full published reopen.
final class ManagedRevision3DialogVoiceSlotRemovalCheckpoint {
  const ManagedRevision3DialogVoiceSlotRemovalCheckpoint._({
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.projectRevision,
    required this.lineId,
    required this.lineRevision,
    required this.localizationId,
    required this.slotId,
    required this.removedSlotRevision,
    required this.locale,
    required this.locId,
    required this.removedTargetResolution,
  });

  final AuthoringWorkingHead head;
  final String projectJson;
  final String projectId;
  final int projectRevision;
  final String lineId;
  final int lineRevision;
  final String localizationId;
  final String slotId;
  final int removedSlotRevision;
  final String locale;
  final String locId;
  final Revision3ContentVoiceTargetResolution removedTargetResolution;
}

/// One retained VoiceTake review-status change returned only after native
/// preparation, full candidate reopen, fixed-head CAS publication, and a full
/// published reopen. The VoiceSlot is unchanged; build remains blocked and
/// runtime remains unqualified.
final class ManagedRevision3VoiceTakeStatusCheckpoint {
  const ManagedRevision3VoiceTakeStatusCheckpoint._({
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.projectRevision,
    required this.lineId,
    required this.localizationId,
    required this.slotId,
    required this.slotRevision,
    required this.locale,
    required this.locId,
    required this.takeId,
    required this.takeRevision,
    required this.previousStatus,
    required this.status,
  });

  final AuthoringWorkingHead head;
  final String projectJson;
  final String projectId;
  final int projectRevision;
  final String lineId;
  final String localizationId;
  final String slotId;
  final int slotRevision;
  final String locale;
  final String locId;
  final String takeId;
  final int takeRevision;
  final AuthoringRevision3VoiceTakeStatus previousStatus;
  final AuthoringRevision3VoiceTakeStatus status;
}

/// One installed-archive Voice target resolution returned only after native
/// evidence was sealed, the candidate was fully reopened, fixed-head CAS
/// published, and the published generation was fully reopened again.
final class ManagedRevision3VoiceTargetCheckpoint {
  ManagedRevision3VoiceTargetCheckpoint._({
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.projectRevision,
    required this.lineId,
    required this.localizationId,
    required this.slotId,
    required this.locale,
    required this.locId,
    required this.resolution,
    required List<AuthoringRevision3VoiceTarget> targets,
    required this.archiveObservation,
  }) : targets = List.unmodifiable(targets);

  final AuthoringWorkingHead head;
  final String projectJson;
  final String projectId;
  final int projectRevision;
  final String lineId;
  final String localizationId;
  final String slotId;
  final String locale;
  final String locId;
  final AuthoringRevision3VoiceTargetResolutionState resolution;
  final List<AuthoringRevision3VoiceTarget> targets;
  final AuthoringRevision3VoiceArchiveObservation? archiveObservation;
}

/// One DataAsset stage returned only after its candidate was fully reopened, fixed-head CAS
/// published, and fully reopened again. It carries no build, runtime, pack, deploy, or native
/// publication claim.
final class ManagedRevision3DataAssetStageCheckpoint {
  const ManagedRevision3DataAssetStageCheckpoint._({
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.projectRevision,
    required this.stage,
    required this.deduplicatedBlobs,
  });

  final AuthoringWorkingHead head;
  final String projectJson;
  final String projectId;
  final int projectRevision;
  final AuthoringRevision3DataAssetStage stage;
  final int deduplicatedBlobs;
}

/// One registry removal returned only after guarded publication and full reopen.
final class ManagedRevision3DataAssetStageRemovalCheckpoint {
  const ManagedRevision3DataAssetStageRemovalCheckpoint._({
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.projectRevision,
    required this.removed,
  });

  final AuthoringWorkingHead head;
  final String projectJson;
  final String projectId;
  final int projectRevision;
  final AuthoringRevision3DataAssetStage removed;
}

/// One exact Story Draft removal returned only after native preparation, full
/// candidate reopen, fixed-head CAS publication, and full published reopen.
/// Its uniquely-owned generated ScriptModule is removed in the same atomic
/// project revision; no build, runtime, artifact, or native publication
/// authority is implied.
final class ManagedRevision3StoryDraftRemovalCheckpoint {
  const ManagedRevision3StoryDraftRemovalCheckpoint._({
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.projectRevision,
    required this.removedDraftId,
    required this.removedDraftKind,
    required this.removedDraftRevision,
    required this.removedScriptModuleId,
    required this.removedScriptModuleRevision,
  });

  final AuthoringWorkingHead head;
  final String projectJson;
  final String projectId;
  final int projectRevision;
  final String removedDraftId;
  final AuthoringStoryDraftKind removedDraftKind;
  final int removedDraftRevision;
  final String removedScriptModuleId;
  final int removedScriptModuleRevision;
}

/// Narrow seam over the native managed-store document API.
///
/// The interface keeps session durability and ordering independently testable;
/// production callers normally use [ModFfiManagedAuthoringStore].
abstract interface class ManagedAuthoringStore {
  Future<AuthoringStoreOpenedResult> open({
    required String root,
    required AuthoringAssetVerification verification,
    required AuthoringValidationProfile profile,
  });

  Future<AuthoringCheckpointPreparation> prepareCheckpoint({
    required String root,
    required AuthoringWorkingHead? expectedHead,
    required String projectJson,
    required AuthoringValidationProfile profile,
  });

  Future<AuthoringStoreOpenedResult> openHeadBytes({
    required String root,
    required AuthoringWorkingHead head,
    required AuthoringAssetVerification verification,
    required AuthoringValidationProfile profile,
  });
}

class ModFfiManagedAuthoringStore implements ManagedAuthoringStore {
  const ModFfiManagedAuthoringStore(this.ffi);

  final ModFfi ffi;

  @override
  Future<AuthoringStoreOpenedResult> open({
    required String root,
    required AuthoringAssetVerification verification,
    required AuthoringValidationProfile profile,
  }) => ffi.authoringStoreOpenDocument(
    root: root,
    verification: verification,
    profile: profile,
  );

  @override
  Future<AuthoringCheckpointPreparation> prepareCheckpoint({
    required String root,
    required AuthoringWorkingHead? expectedHead,
    required String projectJson,
    required AuthoringValidationProfile profile,
  }) => ffi.authoringStorePrepareDocumentCheckpoint(
    root: root,
    expectedHead: expectedHead,
    projectJson: projectJson,
    profile: profile,
  );

  @override
  Future<AuthoringStoreOpenedResult> openHeadBytes({
    required String root,
    required AuthoringWorkingHead head,
    required AuthoringAssetVerification verification,
    required AuthoringValidationProfile profile,
  }) => ffi.authoringStoreOpenHeadBytesDocument(
    root: root,
    head: head,
    verification: verification,
    profile: profile,
  );
}

/// Narrow seam over the dedicated schema-revision-3 managed-store API.
///
/// Revision 3 deliberately has no validation profile, diagnostics, readiness, runtime, deployment,
/// or publication-authority fields. Production callers normally use
/// [ModFfiManagedRevision3AuthoringStore].
abstract interface class ManagedRevision3AuthoringStore {
  Future<AuthoringRevision3StoreOpenedResult> open({
    required String root,
    required AuthoringAssetVerification verification,
  });

  Future<AuthoringRevision3CheckpointPreparation> prepareCheckpoint({
    required String root,
    required AuthoringWorkingHead? expectedHead,
    required String projectJson,
  });

  Future<AuthoringRevision3StoreOpenedResult> openHeadBytes({
    required String root,
    required AuthoringWorkingHead head,
    required AuthoringAssetVerification verification,
  });

  Future<AuthoringRevision3QuestDraftPreparation> prepareQuestDraftV3({
    required String root,
    required String gameRoot,
    required String currentProjectJson,
    required String questRequestJson,
  });

  Future<AuthoringRevision3QuestOutlineEditPreparation>
  prepareQuestOutlineEditV1({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3QuestOutlineEditRequestV1 request,
  });

  Future<AuthoringRevision3QuestOutlineEditPreparationV2>
  prepareQuestOutlineEditV2({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3QuestOutlineEditRequestV2 request,
  });

  Future<AuthoringRevision3QuestTransitionsEditPreparation>
  prepareQuestTransitionsEditV1({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3QuestTransitionsEditRequestV1 request,
  });

  Future<AuthoringRevision3QuestContextEditPreparation>
  prepareQuestContextEditV1({
    required String root,
    required String gameRoot,
    required String currentProjectJson,
    required AuthoringRevision3QuestContextEditRequestV1 request,
  });

  Future<AuthoringRevision3QuestSourceInspectionResult> inspectQuestSourceV1({
    required String root,
    required String gameRoot,
    required AuthoringWorkingHead expectedHead,
    required String questId,
  });

  Future<AuthoringRevision3NpcSourceInspectionResult> inspectNpcSourceV1({
    required String root,
    required AuthoringWorkingHead expectedHead,
    required String npcId,
  });

  Future<AuthoringRevision3ManagedCompilerCheckResult> checkQuestCompilerV1({
    required String root,
    required String gameRoot,
    required AuthoringWorkingHead expectedHead,
    required String questId,
  });

  Future<AuthoringRevision3ManagedCompilerCheckResult> checkNpcCompilerV1({
    required String root,
    required String gameRoot,
    required AuthoringWorkingHead expectedHead,
    required String npcId,
  });

  Future<AuthoringRevision3DataAssetPackageIndexResult>
  readDataAssetPackageIndexV1({
    required String root,
    required String gameRoot,
    required AuthoringWorkingHead expectedHead,
  });

  Future<AuthoringRevision3InstalledDataAssetInspectionResult>
  inspectInstalledDataAssetV1({
    required String root,
    required String gameRoot,
    required AuthoringWorkingHead expectedHead,
    required AuthoringRevision3DataAssetPackageIndexResult expectedSnapshot,
    required AuthoringRevision3DataAssetPackageCandidate candidate,
  });

  Future<AuthoringRevision3DataAssetStagePreparation>
  prepareInstalledDataAssetEditV1({
    required String root,
    required String gameRoot,
    required AuthoringWorkingHead expectedHead,
    required DataAssetInstalledSemanticEditIntent intent,
  });

  Future<AuthoringRevision3DataAssetStagePreparation>
  prepareReviewedInstalledDataAssetEditV1({
    required String root,
    required String gameRoot,
    required AuthoringWorkingHead expectedHead,
    required ReviewedInstalledDataAssetEditIntent intent,
  });

  Future<AuthoringRevision3NpcDraftPreparation> prepareNpcDraftV1({
    required String root,
    required String gameRoot,
    required String currentProjectJson,
    required AuthoringRevision3NpcDraftRequestV1 request,
  });

  Future<AuthoringRevision3DialogLineEntryPreparation> prepareDialogLineV1({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3DialogLineEntryRequestV1 request,
  });

  Future<AuthoringRevision3DialogLocalizationReadResult>
  readDialogLocalizationV1({
    required String root,
    required AuthoringWorkingHead expectedHead,
    required String localizationId,
    required int expectedLocalizationRevision,
    required String expectedLocId,
  });

  Future<AuthoringRevision3DialogLocalizationEditSeed>
  readDialogLocalizationEditSeedV1({
    required String root,
    required AuthoringWorkingHead expectedHead,
    required String localizationId,
    required int expectedLocalizationRevision,
    required String expectedLocId,
  });

  Future<AuthoringRevision3DialogLocalizationEditPreparation>
  prepareDialogLocalizationEditV1({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3DialogLocalizationEditRequestV1 request,
  });

  Future<AuthoringRevision3VoiceTakePreparation> prepareVoiceTakeV1({
    required String root,
    required String gameRoot,
    required String source,
    required String currentProjectJson,
    required AuthoringRevision3VoiceTakeRequestV1 request,
  });

  Future<AuthoringRevision3VoiceTakeSelectionPreparation>
  prepareVoiceTakeSelectionV1({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3VoiceTakeSelectionRequestV1 request,
  });

  Future<AuthoringRevision3VoiceTakeStatusPreparation>
  prepareVoiceTakeStatusV1({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3VoiceTakeStatusRequestV1 request,
  });

  Future<AuthoringRevision3VoiceTargetPreparation> prepareVoiceTargetV1({
    required String root,
    required String gameRoot,
    required String currentProjectJson,
    required AuthoringRevision3VoiceTargetRequestV1 request,
  });

  Future<AuthoringRevision3VoiceBuildPlanResult> planVoiceV1({
    required String root,
    required String currentProjectJson,
    required AuthoringWorkingHead expectedHead,
  });

  Future<AuthoringRevision3VoiceBuildResult> buildVoiceV1({
    required String root,
    required String gameRoot,
    required String currentProjectJson,
    required AuthoringWorkingHead expectedHead,
    required String output,
  });

  Future<AuthoringRevision3ContentIndexResult> readContentIndex({
    required String root,
    required AuthoringWorkingHead expectedHead,
  });

  Future<AuthoringRevision3DataAssetStagePreparation> prepareDataAssetStageV1({
    required String root,
    required AuthoringWorkingHead expectedHead,
    required String patchReceiptPath,
  });

  Future<AuthoringRevision3DataAssetStagePreparation> prepareDataAssetEditV1({
    required String root,
    required AuthoringWorkingHead expectedHead,
    required DataAssetSemanticEditIntent intent,
  });

  Future<AuthoringRevision3DataAssetStageListResult> listDataAssetStagesV1({
    required String root,
    required AuthoringWorkingHead expectedHead,
  });

  Future<AuthoringRevision3DataAssetStageRemovalPreparation>
  prepareRemoveDataAssetStageV1({
    required String root,
    required AuthoringWorkingHead expectedHead,
    required String targetPath,
  });
}

/// Narrow capability for producing one immutable reviewed DataAsset pack.
/// Keeping it separate avoids forcing checkpoint-only test stores to claim
/// build authority they do not implement.
abstract interface class ManagedRevision3ReviewedDataAssetBuildStore {
  Future<AuthoringRevision3ReviewedDataAssetBuildResult>
  buildReviewedDataAssetV1({
    required String root,
    required String gameRoot,
    required String currentProjectJson,
    required AuthoringWorkingHead expectedHead,
    required String targetPath,
    required String packName,
    required String output,
  });
}

/// Narrow capability for exporting an immutable, exact-basis revision-3
/// project snapshot. Keeping it separate lets checkpoint-only stores remain
/// honest about the operations they implement.
abstract interface class ManagedRevision3ExactSnapshotExportStore {
  Future<AuthoringRevision3ExactSnapshotExportResult> exportExactSnapshotV1({
    required String root,
    required AuthoringWorkingHead expectedHead,
    required String output,
  });
}

/// Optional authenticated-history capability. Keeping this separate prevents
/// checkpoint-only alternate stores and test doubles from accidentally
/// claiming restore authority.
abstract interface class ManagedRevision3ProjectHistoryStore {
  Future<AuthoringRevision3ProjectHistoryResult> listProjectHistoryV1({
    required String root,
    required AuthoringWorkingHead expectedHead,
  });

  Future<AuthoringRevision3ProjectHistoryRestorePreparation>
  prepareProjectHistoryRestoreV1({
    required String root,
    required AuthoringWorkingHead expectedHead,
    required AuthoringWorkingHead targetHead,
  });
}

/// Optional native authority for one read-only Voice folder plan and one
/// all-or-nothing unpublished batch candidate. Older stores and unrelated
/// fakes must opt in explicitly before they can inspect folders or mutate a
/// project through this path.
abstract interface class ManagedRevision3VoiceBatchStore {
  Future<AuthoringRevision3VoiceBatchPlanResult> planVoiceBatchV1({
    required String root,
    required String gameRoot,
    required String sourceFolder,
    required String locale,
    required String currentProjectJson,
    required AuthoringWorkingHead expectedHead,
  });

  Future<AuthoringRevision3VoiceBatchPreparation> prepareVoiceBatchV1({
    required String root,
    required String gameRoot,
    required String sourceFolder,
    required String currentProjectJson,
    required AuthoringRevision3VoiceBatchPlanResult plan,
  });
}

/// Optional authority for preparing one project-only Quest transcript
/// candidate. Alternate checkpoint stores and unrelated fakes must opt in.
abstract interface class ManagedRevision3QuestTranscriptStore {
  Future<AuthoringRevision3QuestTranscriptPreparation>
  prepareQuestTranscriptV1({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3QuestTranscriptRequestV1 request,
  });
}

/// Optional authority for preparing one project-only NPC greeting candidate.
/// Alternate checkpoint stores and unrelated fakes must opt in explicitly.
abstract interface class ManagedRevision3NpcGreetingStore {
  Future<AuthoringRevision3NpcGreetingPreparation> prepareNpcGreetingV1({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3NpcGreetingRequestV1 request,
  });
}

/// Narrow capability for preparing the exact removal of one Story Draft and
/// its uniquely-owned generated ScriptModule. Keeping it separate avoids
/// granting deletion authority to checkpoint-only alternate stores and fakes.
abstract interface class ManagedRevision3StoryDraftRemovalStore {
  Future<AuthoringRevision3StoryDraftRemovalPreparation>
  prepareRemoveStoryDraftV1({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3StoryDraftRemovalRequestV1 request,
  });
}

/// Narrow read-only capability for copying one exact managed CAS VoiceTake
/// into a native-owned ephemeral preview directory.
abstract interface class ManagedRevision3VoiceTakePreviewStore {
  Future<AuthoringRevision3VoiceTakePreviewRegistration>
  registerVoiceTakePreviewV1({required String root});

  Future<AuthoringRevision3VoiceTakePreviewMaterialization>
  materializeVoiceTakePreviewV1({
    required String root,
    required String cleanupToken,
    required String previewRoot,
    required AuthoringRevision3VoiceTakePreviewRequestV1 request,
  });

  Future<void> releaseVoiceTakePreviewV1({required String cleanupToken});
}

/// Narrow pathless read-only capability for exact managed VoiceTake media QA.
/// Alternate checkpoint stores must opt in and receive no materialization or
/// mutation authority by implementing it.
abstract interface class ManagedRevision3VoiceTakeMediaQaStore {
  Future<AuthoringRevision3VoiceTakeMediaQaResult> inspectVoiceTakeMediaV1({
    required String root,
    required AuthoringRevision3VoiceTakePreviewRequestV1 request,
  });
}

/// Narrow capability for atomically detaching one exact VoiceTake candidate.
/// Separate capability discovery avoids granting deletion authority to
/// checkpoint-only alternate stores and test fakes.
abstract interface class ManagedRevision3VoiceTakeRemovalStore {
  Future<AuthoringRevision3VoiceTakeRemovalPreparation>
  prepareVoiceTakeRemovalV1({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3VoiceTakeRemovalRequestV1 request,
  });
}

/// Narrow capability for deleting one exact empty and unselected dialog
/// VoiceSlot. Alternate checkpoint stores receive no implicit deletion power.
abstract interface class ManagedRevision3DialogVoiceSlotRemovalStore {
  Future<AuthoringRevision3DialogVoiceSlotRemovalPreparation>
  prepareDialogVoiceSlotRemovalV1({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3DialogVoiceSlotRemovalRequestV1 request,
  });
}

/// Narrow capability for preparing one existing NPC name/archetype edit.
/// Checkpoint-only alternate stores do not gain this mutation authority.
abstract interface class ManagedRevision3NpcProfileEditStore {
  Future<AuthoringRevision3NpcProfileEditPreparation> prepareNpcProfileEditV1({
    required String root,
    required String gameRoot,
    required String currentProjectJson,
    required AuthoringRevision3NpcProfileEditRequestV1 request,
  });
}

final class ModFfiManagedRevision3AuthoringStore
    implements
        ManagedRevision3AuthoringStore,
        ManagedRevision3ReviewedDataAssetBuildStore,
        ManagedRevision3ExactSnapshotExportStore,
        ManagedRevision3ProjectHistoryStore,
        ManagedRevision3VoiceBatchStore,
        ManagedRevision3QuestTranscriptStore,
        ManagedRevision3NpcGreetingStore,
        ManagedRevision3StoryDraftRemovalStore,
        ManagedRevision3VoiceTakeMediaQaStore,
        ManagedRevision3VoiceTakePreviewStore,
        ManagedRevision3VoiceTakeRemovalStore,
        ManagedRevision3DialogVoiceSlotRemovalStore,
        ManagedRevision3NpcProfileEditStore {
  const ModFfiManagedRevision3AuthoringStore(this.ffi);

  final ModFfi ffi;

  @override
  Future<AuthoringRevision3StoreOpenedResult> open({
    required String root,
    required AuthoringAssetVerification verification,
  }) => ffi.authoringStoreOpenRevision3(root: root, verification: verification);

  @override
  Future<AuthoringRevision3CheckpointPreparation> prepareCheckpoint({
    required String root,
    required AuthoringWorkingHead? expectedHead,
    required String projectJson,
  }) => ffi.authoringStorePrepareRevision3Checkpoint(
    root: root,
    expectedHead: expectedHead,
    projectJson: projectJson,
  );

  @override
  Future<AuthoringRevision3StoreOpenedResult> openHeadBytes({
    required String root,
    required AuthoringWorkingHead head,
    required AuthoringAssetVerification verification,
  }) => ffi.authoringStoreOpenRevision3HeadBytes(
    root: root,
    head: head,
    verification: verification,
  );

  @override
  Future<AuthoringRevision3QuestDraftPreparation> prepareQuestDraftV3({
    required String root,
    required String gameRoot,
    required String currentProjectJson,
    required String questRequestJson,
  }) => ffi.authoringStorePrepareRevision3QuestDraftV3(
    root: root,
    gameRoot: gameRoot,
    currentProjectJson: currentProjectJson,
    questRequestJson: questRequestJson,
  );

  @override
  Future<AuthoringRevision3QuestOutlineEditPreparation>
  prepareQuestOutlineEditV1({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3QuestOutlineEditRequestV1 request,
  }) => ffi.authoringStorePrepareRevision3QuestOutlineEditV1(
    root: root,
    currentProjectJson: currentProjectJson,
    request: request,
  );

  @override
  Future<AuthoringRevision3NpcProfileEditPreparation> prepareNpcProfileEditV1({
    required String root,
    required String gameRoot,
    required String currentProjectJson,
    required AuthoringRevision3NpcProfileEditRequestV1 request,
  }) => ffi.authoringStorePrepareRevision3NpcProfileEditV1(
    root: root,
    gameRoot: gameRoot,
    currentProjectJson: currentProjectJson,
    request: request,
  );

  @override
  Future<AuthoringRevision3QuestOutlineEditPreparationV2>
  prepareQuestOutlineEditV2({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3QuestOutlineEditRequestV2 request,
  }) => ffi.authoringStorePrepareRevision3QuestOutlineEditV2(
    root: root,
    currentProjectJson: currentProjectJson,
    request: request,
  );

  @override
  Future<AuthoringRevision3QuestTranscriptPreparation>
  prepareQuestTranscriptV1({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3QuestTranscriptRequestV1 request,
  }) => ffi.authoringStorePrepareRevision3QuestTranscriptV1(
    root: root,
    currentProjectJson: currentProjectJson,
    request: request,
  );

  @override
  Future<AuthoringRevision3NpcGreetingPreparation> prepareNpcGreetingV1({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3NpcGreetingRequestV1 request,
  }) => ffi.authoringStorePrepareRevision3NpcGreetingV1(
    root: root,
    currentProjectJson: currentProjectJson,
    request: request,
  );

  @override
  Future<AuthoringRevision3QuestTransitionsEditPreparation>
  prepareQuestTransitionsEditV1({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3QuestTransitionsEditRequestV1 request,
  }) => ffi.authoringStorePrepareRevision3QuestTransitionsEditV1(
    root: root,
    currentProjectJson: currentProjectJson,
    request: request,
  );

  @override
  Future<AuthoringRevision3QuestContextEditPreparation>
  prepareQuestContextEditV1({
    required String root,
    required String gameRoot,
    required String currentProjectJson,
    required AuthoringRevision3QuestContextEditRequestV1 request,
  }) => ffi.authoringStorePrepareRevision3QuestContextEditV1(
    root: root,
    gameRoot: gameRoot,
    currentProjectJson: currentProjectJson,
    request: request,
  );

  @override
  Future<AuthoringRevision3QuestSourceInspectionResult> inspectQuestSourceV1({
    required String root,
    required String gameRoot,
    required AuthoringWorkingHead expectedHead,
    required String questId,
  }) => ffi.authoringStoreInspectRevision3QuestSourceV1(
    root: root,
    gameRoot: gameRoot,
    expectedHead: expectedHead,
    questId: questId,
  );

  @override
  Future<AuthoringRevision3NpcSourceInspectionResult> inspectNpcSourceV1({
    required String root,
    required AuthoringWorkingHead expectedHead,
    required String npcId,
  }) => ffi.authoringStoreInspectRevision3NpcSourceV1(
    root: root,
    expectedHead: expectedHead,
    npcId: npcId,
  );

  @override
  Future<AuthoringRevision3ManagedCompilerCheckResult> checkQuestCompilerV1({
    required String root,
    required String gameRoot,
    required AuthoringWorkingHead expectedHead,
    required String questId,
  }) => ffi.authoringStoreCheckRevision3QuestCompilerV1(
    root: root,
    gameRoot: gameRoot,
    expectedHead: expectedHead,
    questId: questId,
  );

  @override
  Future<AuthoringRevision3ManagedCompilerCheckResult> checkNpcCompilerV1({
    required String root,
    required String gameRoot,
    required AuthoringWorkingHead expectedHead,
    required String npcId,
  }) => ffi.authoringStoreCheckRevision3NpcCompilerV1(
    root: root,
    gameRoot: gameRoot,
    expectedHead: expectedHead,
    npcId: npcId,
  );

  @override
  Future<AuthoringRevision3DataAssetPackageIndexResult>
  readDataAssetPackageIndexV1({
    required String root,
    required String gameRoot,
    required AuthoringWorkingHead expectedHead,
  }) => ffi.authoringStoreReadRevision3DataAssetPackageIndexV1(
    root: root,
    gameRoot: gameRoot,
    expectedHead: expectedHead,
  );

  @override
  Future<AuthoringRevision3InstalledDataAssetInspectionResult>
  inspectInstalledDataAssetV1({
    required String root,
    required String gameRoot,
    required AuthoringWorkingHead expectedHead,
    required AuthoringRevision3DataAssetPackageIndexResult expectedSnapshot,
    required AuthoringRevision3DataAssetPackageCandidate candidate,
  }) => ffi.authoringStoreInspectRevision3InstalledDataAssetV1(
    root: root,
    gameRoot: gameRoot,
    expectedHead: expectedHead,
    expectedSnapshot: expectedSnapshot,
    candidate: candidate,
  );

  @override
  Future<AuthoringRevision3DataAssetStagePreparation>
  prepareInstalledDataAssetEditV1({
    required String root,
    required String gameRoot,
    required AuthoringWorkingHead expectedHead,
    required DataAssetInstalledSemanticEditIntent intent,
  }) => ffi.authoringStorePrepareRevision3InstalledDataAssetEditV1(
    root: root,
    gameRoot: gameRoot,
    expectedHead: expectedHead,
    intent: intent,
  );

  @override
  Future<AuthoringRevision3DataAssetStagePreparation>
  prepareReviewedInstalledDataAssetEditV1({
    required String root,
    required String gameRoot,
    required AuthoringWorkingHead expectedHead,
    required ReviewedInstalledDataAssetEditIntent intent,
  }) => ffi.authoringStorePrepareRevision3ReviewedInstalledDataAssetEditV1(
    root: root,
    gameRoot: gameRoot,
    expectedHead: expectedHead,
    intent: intent,
  );

  @override
  Future<AuthoringRevision3NpcDraftPreparation> prepareNpcDraftV1({
    required String root,
    required String gameRoot,
    required String currentProjectJson,
    required AuthoringRevision3NpcDraftRequestV1 request,
  }) => ffi.authoringStorePrepareRevision3NpcDraftV1(
    root: root,
    gameRoot: gameRoot,
    currentProjectJson: currentProjectJson,
    request: request,
  );

  @override
  Future<AuthoringRevision3DialogLineEntryPreparation> prepareDialogLineV1({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3DialogLineEntryRequestV1 request,
  }) => ffi.authoringStorePrepareRevision3DialogLineV1(
    root: root,
    currentProjectJson: currentProjectJson,
    request: request,
  );

  @override
  Future<AuthoringRevision3DialogLocalizationReadResult>
  readDialogLocalizationV1({
    required String root,
    required AuthoringWorkingHead expectedHead,
    required String localizationId,
    required int expectedLocalizationRevision,
    required String expectedLocId,
  }) => ffi.authoringStoreReadRevision3DialogLocalizationV1(
    root: root,
    expectedHead: expectedHead,
    localizationId: localizationId,
    expectedLocalizationRevision: expectedLocalizationRevision,
    expectedLocId: expectedLocId,
  );

  @override
  Future<AuthoringRevision3DialogLocalizationEditSeed>
  readDialogLocalizationEditSeedV1({
    required String root,
    required AuthoringWorkingHead expectedHead,
    required String localizationId,
    required int expectedLocalizationRevision,
    required String expectedLocId,
  }) => ffi.authoringStoreReadRevision3DialogLocalizationEditSeedV1(
    root: root,
    expectedHead: expectedHead,
    localizationId: localizationId,
    expectedLocalizationRevision: expectedLocalizationRevision,
    expectedLocId: expectedLocId,
  );

  @override
  Future<AuthoringRevision3DialogLocalizationEditPreparation>
  prepareDialogLocalizationEditV1({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3DialogLocalizationEditRequestV1 request,
  }) => ffi.authoringStorePrepareRevision3DialogLocalizationEditV1(
    root: root,
    currentProjectJson: currentProjectJson,
    request: request,
  );

  @override
  Future<AuthoringRevision3VoiceTakePreparation> prepareVoiceTakeV1({
    required String root,
    required String gameRoot,
    required String source,
    required String currentProjectJson,
    required AuthoringRevision3VoiceTakeRequestV1 request,
  }) => ffi.authoringStorePrepareRevision3VoiceTakeV1(
    root: root,
    gameRoot: gameRoot,
    source: source,
    currentProjectJson: currentProjectJson,
    request: request,
  );

  @override
  Future<AuthoringRevision3VoiceTakePreviewRegistration>
  registerVoiceTakePreviewV1({required String root}) =>
      ffi.authoringStoreRegisterRevision3VoiceTakePreviewV1(root: root);

  @override
  Future<AuthoringRevision3VoiceTakeMediaQaResult> inspectVoiceTakeMediaV1({
    required String root,
    required AuthoringRevision3VoiceTakePreviewRequestV1 request,
  }) => ffi.authoringStoreInspectRevision3VoiceTakeMediaV1(
    root: root,
    request: request,
  );

  @override
  Future<AuthoringRevision3VoiceTakePreviewMaterialization>
  materializeVoiceTakePreviewV1({
    required String root,
    required String cleanupToken,
    required String previewRoot,
    required AuthoringRevision3VoiceTakePreviewRequestV1 request,
  }) => ffi.authoringStoreMaterializeRevision3VoiceTakePreviewV1(
    root: root,
    cleanupToken: cleanupToken,
    previewRoot: previewRoot,
    request: request,
  );

  @override
  Future<void> releaseVoiceTakePreviewV1({required String cleanupToken}) =>
      ffi.authoringStoreReleaseRevision3VoiceTakePreviewV1(
        cleanupToken: cleanupToken,
      );

  @override
  Future<AuthoringRevision3VoiceBatchPlanResult> planVoiceBatchV1({
    required String root,
    required String gameRoot,
    required String sourceFolder,
    required String locale,
    required String currentProjectJson,
    required AuthoringWorkingHead expectedHead,
  }) => ffi.authoringStorePlanRevision3VoiceBatchV1(
    root: root,
    gameRoot: gameRoot,
    sourceFolder: sourceFolder,
    locale: locale,
    currentProjectJson: currentProjectJson,
    expectedHead: expectedHead,
  );

  @override
  Future<AuthoringRevision3VoiceBatchPreparation> prepareVoiceBatchV1({
    required String root,
    required String gameRoot,
    required String sourceFolder,
    required String currentProjectJson,
    required AuthoringRevision3VoiceBatchPlanResult plan,
  }) => ffi.authoringStorePrepareRevision3VoiceBatchV1(
    root: root,
    gameRoot: gameRoot,
    sourceFolder: sourceFolder,
    currentProjectJson: currentProjectJson,
    plan: plan,
  );

  @override
  Future<AuthoringRevision3VoiceTakeSelectionPreparation>
  prepareVoiceTakeSelectionV1({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3VoiceTakeSelectionRequestV1 request,
  }) => ffi.authoringStorePrepareRevision3VoiceTakeSelectionV1(
    root: root,
    currentProjectJson: currentProjectJson,
    request: request,
  );

  @override
  Future<AuthoringRevision3VoiceTakeRemovalPreparation>
  prepareVoiceTakeRemovalV1({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3VoiceTakeRemovalRequestV1 request,
  }) => ffi.authoringStorePrepareRevision3VoiceTakeRemovalV1(
    root: root,
    currentProjectJson: currentProjectJson,
    request: request,
  );

  @override
  Future<AuthoringRevision3DialogVoiceSlotRemovalPreparation>
  prepareDialogVoiceSlotRemovalV1({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3DialogVoiceSlotRemovalRequestV1 request,
  }) => ffi.authoringStorePrepareRevision3DialogVoiceSlotRemovalV1(
    root: root,
    currentProjectJson: currentProjectJson,
    request: request,
  );

  @override
  Future<AuthoringRevision3VoiceTakeStatusPreparation>
  prepareVoiceTakeStatusV1({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3VoiceTakeStatusRequestV1 request,
  }) => ffi.authoringStorePrepareRevision3VoiceTakeStatusV1(
    root: root,
    currentProjectJson: currentProjectJson,
    request: request,
  );

  @override
  Future<AuthoringRevision3VoiceTargetPreparation> prepareVoiceTargetV1({
    required String root,
    required String gameRoot,
    required String currentProjectJson,
    required AuthoringRevision3VoiceTargetRequestV1 request,
  }) => ffi.authoringStorePrepareRevision3VoiceTargetV1(
    root: root,
    gameRoot: gameRoot,
    currentProjectJson: currentProjectJson,
    request: request,
  );

  @override
  Future<AuthoringRevision3VoiceBuildPlanResult> planVoiceV1({
    required String root,
    required String currentProjectJson,
    required AuthoringWorkingHead expectedHead,
  }) => ffi.authoringStorePlanRevision3VoiceV1(
    root: root,
    currentProjectJson: currentProjectJson,
    expectedHead: expectedHead,
  );

  @override
  Future<AuthoringRevision3VoiceBuildResult> buildVoiceV1({
    required String root,
    required String gameRoot,
    required String currentProjectJson,
    required AuthoringWorkingHead expectedHead,
    required String output,
  }) => ffi.authoringStoreBuildRevision3VoiceV1(
    root: root,
    gameRoot: gameRoot,
    currentProjectJson: currentProjectJson,
    expectedHead: expectedHead,
    output: output,
  );

  @override
  Future<AuthoringRevision3ExactSnapshotExportResult> exportExactSnapshotV1({
    required String root,
    required AuthoringWorkingHead expectedHead,
    required String output,
  }) => ffi.authoringStoreExportRevision3ExactSnapshotV1(
    root: root,
    expectedHead: expectedHead,
    output: output,
  );

  @override
  Future<AuthoringRevision3ProjectHistoryResult> listProjectHistoryV1({
    required String root,
    required AuthoringWorkingHead expectedHead,
  }) => ffi.authoringStoreListRevision3HistoryV1(
    root: root,
    expectedHead: expectedHead,
  );

  @override
  Future<AuthoringRevision3ProjectHistoryRestorePreparation>
  prepareProjectHistoryRestoreV1({
    required String root,
    required AuthoringWorkingHead expectedHead,
    required AuthoringWorkingHead targetHead,
  }) => ffi.authoringStorePrepareRevision3HistoryRestoreV1(
    root: root,
    expectedHead: expectedHead,
    targetHead: targetHead,
  );

  @override
  Future<AuthoringRevision3ReviewedDataAssetBuildResult>
  buildReviewedDataAssetV1({
    required String root,
    required String gameRoot,
    required String currentProjectJson,
    required AuthoringWorkingHead expectedHead,
    required String targetPath,
    required String packName,
    required String output,
  }) => ffi.authoringStoreBuildRevision3ReviewedDataAssetV1(
    root: root,
    gameRoot: gameRoot,
    currentProjectJson: currentProjectJson,
    expectedHead: expectedHead,
    targetPath: targetPath,
    packName: packName,
    output: output,
  );

  @override
  Future<AuthoringRevision3ContentIndexResult> readContentIndex({
    required String root,
    required AuthoringWorkingHead expectedHead,
  }) => ffi.authoringStoreReadRevision3ContentIndexV1(
    root: root,
    expectedHead: expectedHead,
  );

  @override
  Future<AuthoringRevision3DataAssetStagePreparation> prepareDataAssetStageV1({
    required String root,
    required AuthoringWorkingHead expectedHead,
    required String patchReceiptPath,
  }) => ffi.authoringStorePrepareRevision3DataAssetStageV1(
    root: root,
    expectedHead: expectedHead,
    patchReceiptPath: patchReceiptPath,
  );

  @override
  Future<AuthoringRevision3DataAssetStagePreparation> prepareDataAssetEditV1({
    required String root,
    required AuthoringWorkingHead expectedHead,
    required DataAssetSemanticEditIntent intent,
  }) => ffi.authoringStorePrepareRevision3DataAssetEditV1(
    root: root,
    expectedHead: expectedHead,
    intent: intent,
  );

  @override
  Future<AuthoringRevision3DataAssetStageListResult> listDataAssetStagesV1({
    required String root,
    required AuthoringWorkingHead expectedHead,
  }) => ffi.authoringStoreListRevision3DataAssetStagesV1(
    root: root,
    expectedHead: expectedHead,
  );

  @override
  Future<AuthoringRevision3DataAssetStageRemovalPreparation>
  prepareRemoveDataAssetStageV1({
    required String root,
    required AuthoringWorkingHead expectedHead,
    required String targetPath,
  }) => ffi.authoringStorePrepareRemoveRevision3DataAssetStageV1(
    root: root,
    expectedHead: expectedHead,
    targetPath: targetPath,
  );

  @override
  Future<AuthoringRevision3StoryDraftRemovalPreparation>
  prepareRemoveStoryDraftV1({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3StoryDraftRemovalRequestV1 request,
  }) => ffi.authoringStorePrepareRemoveRevision3StoryDraftV1(
    root: root,
    currentProjectJson: currentProjectJson,
    request: request,
  );
}

final class _ManagedOpenedCheckpoint {
  const _ManagedOpenedCheckpoint({
    required this.head,
    required this.projectJson,
    this.diagnostics,
    this.blocksBuild,
    this.projectId,
    this.projectRevision,
  });

  final AuthoringWorkingHead head;
  final String projectJson;
  final List<AuthoringDiagnostic>? diagnostics;
  final bool? blocksBuild;
  final String? projectId;
  final int? projectRevision;
}

final class _ManagedPreparedCheckpoint<T> {
  const _ManagedPreparedCheckpoint({
    required this.head,
    required this.projectJson,
    required this.value,
  });

  final AuthoringWorkingHead head;
  final String projectJson;
  final T value;
}

/// A History operation failed after the exact current checkpoint was independently
/// fully reopened. The failure is therefore scoped to the retained History
/// capability and must not poison otherwise-valid project authoring.
final class _ManagedRevision3HistoryFailureWithVerifiedCurrent {
  const _ManagedRevision3HistoryFailureWithVerifiedCurrent(
    this.error,
    this.stackTrace,
  );

  final ModFfiException error;
  final StackTrace stackTrace;
}

abstract interface class _ManagedCheckpointStore {
  bool get supportsReviewedDataAssetBuild;

  Future<_ManagedOpenedCheckpoint> open({
    required String root,
    required AuthoringAssetVerification verification,
  });

  Future<AuthoringWorkingHead> prepareCheckpoint({
    required String root,
    required AuthoringWorkingHead? expectedHead,
    required String projectJson,
  });

  Future<_ManagedOpenedCheckpoint> openHeadBytes({
    required String root,
    required AuthoringWorkingHead head,
    required AuthoringAssetVerification verification,
  });

  Future<AuthoringRevision3ReviewedDataAssetBuildResult>
  buildReviewedDataAssetV1({
    required String root,
    required String gameRoot,
    required String currentProjectJson,
    required AuthoringWorkingHead expectedHead,
    required String targetPath,
    required String packName,
    required String output,
  });
}

final class _Revision12ManagedCheckpointStore
    implements _ManagedCheckpointStore {
  const _Revision12ManagedCheckpointStore(this.store, this.profile);

  final ManagedAuthoringStore store;
  final AuthoringValidationProfile profile;

  @override
  bool get supportsReviewedDataAssetBuild => false;

  @override
  Future<_ManagedOpenedCheckpoint> open({
    required String root,
    required AuthoringAssetVerification verification,
  }) async => _fromOpened(
    await store.open(root: root, verification: verification, profile: profile),
  );

  @override
  Future<AuthoringWorkingHead> prepareCheckpoint({
    required String root,
    required AuthoringWorkingHead? expectedHead,
    required String projectJson,
  }) async {
    final prepared = await store.prepareCheckpoint(
      root: root,
      expectedHead: expectedHead,
      projectJson: projectJson,
      profile: profile,
    );
    return prepared.head;
  }

  @override
  Future<_ManagedOpenedCheckpoint> openHeadBytes({
    required String root,
    required AuthoringWorkingHead head,
    required AuthoringAssetVerification verification,
  }) async => _fromOpened(
    await store.openHeadBytes(
      root: root,
      head: head,
      verification: verification,
      profile: profile,
    ),
  );

  @override
  Future<AuthoringRevision3ReviewedDataAssetBuildResult>
  buildReviewedDataAssetV1({
    required String root,
    required String gameRoot,
    required String currentProjectJson,
    required AuthoringWorkingHead expectedHead,
    required String targetPath,
    required String packName,
    required String output,
  }) => throw UnsupportedError(
    'reviewed DataAsset builds require a managed revision-3 Store',
  );

  static _ManagedOpenedCheckpoint _fromOpened(
    AuthoringStoreOpenedResult opened,
  ) => _ManagedOpenedCheckpoint(
    head: opened.head,
    projectJson: opened.projectJson,
    diagnostics: opened.diagnostics,
    blocksBuild: opened.blocksBuild,
  );
}

final class _Revision3ManagedCheckpointStore
    implements _ManagedCheckpointStore {
  const _Revision3ManagedCheckpointStore(this.store);

  final ManagedRevision3AuthoringStore store;

  @override
  bool get supportsReviewedDataAssetBuild =>
      store is ManagedRevision3ReviewedDataAssetBuildStore;

  @override
  Future<_ManagedOpenedCheckpoint> open({
    required String root,
    required AuthoringAssetVerification verification,
  }) async =>
      _fromOpened(await store.open(root: root, verification: verification));

  @override
  Future<AuthoringWorkingHead> prepareCheckpoint({
    required String root,
    required AuthoringWorkingHead? expectedHead,
    required String projectJson,
  }) async {
    final prepared = await store.prepareCheckpoint(
      root: root,
      expectedHead: expectedHead,
      projectJson: projectJson,
    );
    return prepared.head;
  }

  @override
  Future<_ManagedOpenedCheckpoint> openHeadBytes({
    required String root,
    required AuthoringWorkingHead head,
    required AuthoringAssetVerification verification,
  }) async => _fromOpened(
    await store.openHeadBytes(
      root: root,
      head: head,
      verification: verification,
    ),
  );

  @override
  Future<AuthoringRevision3ReviewedDataAssetBuildResult>
  buildReviewedDataAssetV1({
    required String root,
    required String gameRoot,
    required String currentProjectJson,
    required AuthoringWorkingHead expectedHead,
    required String targetPath,
    required String packName,
    required String output,
  }) {
    final buildStore = store;
    if (!supportsReviewedDataAssetBuild ||
        buildStore is! ManagedRevision3ReviewedDataAssetBuildStore) {
      throw UnsupportedError(
        'this managed revision-3 Store has no reviewed DataAsset build capability',
      );
    }
    return (buildStore as ManagedRevision3ReviewedDataAssetBuildStore)
        .buildReviewedDataAssetV1(
          root: root,
          gameRoot: gameRoot,
          currentProjectJson: currentProjectJson,
          expectedHead: expectedHead,
          targetPath: targetPath,
          packName: packName,
          output: output,
        );
  }

  static _ManagedOpenedCheckpoint _fromOpened(
    AuthoringRevision3StoreOpenedResult opened,
  ) => _ManagedOpenedCheckpoint(
    head: opened.head,
    projectJson: opened.projectJson,
    projectId: opened.projectId,
    projectRevision: opened.projectRevision,
  );
}

/// Exclusive, crash-recoverable editing session for one closed schema-revision-1/2 format-2
/// working tree.
///
/// Immutable objects are prepared by the native store. The only Dart-owned mutation is
/// publication of the fixed `gore-project.json` head. Publication is an exact byte-for-byte CAS
/// and every candidate, repaired generation, and published generation is reopened using full
/// asset verification.
class ManagedAuthoringProjectSession {
  ManagedAuthoringProjectSession._(this._core, this._profile);

  final _ManagedProjectSessionCore _core;
  final AuthoringValidationProfile _profile;

  Directory get root => _core.root;
  String get projectJson => _core.projectJson;
  AuthoringWorkingHead get head => _core.head;
  List<AuthoringDiagnostic> get diagnostics => _core._opened.diagnostics!;
  bool get blocksBuild => _core._opened.blocksBuild!;
  AuthoringValidationProfile get profile => _profile;
  bool get isClosed => _core.isClosed;
  bool get requiresReopen => _core.requiresReopen;
  File get headFile => _core.headFile;

  static Future<ManagedAuthoringProjectSession> create({
    required Directory root,
    required ManagedAuthoringStore store,
    required String projectJson,
    required AuthoringValidationProfile profile,
    AtomicByteReplacement? replacement,
  }) async => ManagedAuthoringProjectSession._(
    await _ManagedProjectSessionCore.create(
      root: root,
      store: _Revision12ManagedCheckpointStore(store, profile),
      projectJson: projectJson,
      replacement: replacement,
    ),
    profile,
  );

  static Future<ManagedAuthoringProjectSession> open({
    required Directory root,
    required ManagedAuthoringStore store,
    required AuthoringValidationProfile profile,
    AtomicByteReplacement? replacement,
  }) async => ManagedAuthoringProjectSession._(
    await _ManagedProjectSessionCore.open(
      root: root,
      store: _Revision12ManagedCheckpointStore(store, profile),
      replacement: replacement,
    ),
    profile,
  );

  Future<void> save(String projectJson) => _core.save(projectJson);

  Future<T> deriveAndSave<T>(ManagedProjectDeriver<T> derive) =>
      _core.deriveAndSave(derive);

  /// Reopen the exact currently-published checkpoint with full asset
  /// verification without preparing or publishing a new checkpoint.
  Future<void> verifyCurrentHead() => _core.verifyCurrentHead();

  Future<void> close() => _core.close();
}

/// Safe managed session for a canonical schema-revision-3 format-2 working tree.
///
/// This API exposes only durable checkpoint identity. Revision 3 store responses do not carry
/// diagnostics, build readiness, runtime compatibility, deployment status, or publication
/// authority, so this session intentionally does not synthesize or expose any of those claims.
/// It otherwise uses the exact same lock, serialized operation lane, compare-and-swap,
/// verification, repair, and no-clobber publication core as [ManagedAuthoringProjectSession].
class ManagedRevision3AuthoringProjectSession {
  ManagedRevision3AuthoringProjectSession._(this._core, this._store);

  final _ManagedProjectSessionCore _core;
  final ManagedRevision3AuthoringStore _store;

  Directory get root => _core.root;
  String get projectJson => _core.projectJson;
  AuthoringWorkingHead get head => _core.head;
  String get projectId => _core._opened.projectId!;
  int get projectRevision => _core._opened.projectRevision!;
  bool get isClosed => _core.isClosed;
  bool get requiresReopen => _core.requiresReopen;
  bool get supportsReviewedDataAssetBuild =>
      _core.supportsReviewedDataAssetBuild;
  bool get supportsExactSnapshotExport =>
      _store is ManagedRevision3ExactSnapshotExportStore;
  bool get supportsStoryDraftRemoval =>
      _store is ManagedRevision3StoryDraftRemovalStore;
  bool get supportsVoiceTakePreview =>
      _store is ManagedRevision3VoiceTakePreviewStore;
  bool get supportsVoiceTakeMediaQa =>
      _store is ManagedRevision3VoiceTakeMediaQaStore;
  bool get supportsVoiceTakeRemoval =>
      _store is ManagedRevision3VoiceTakeRemovalStore;
  bool get supportsVoiceBatch => _store is ManagedRevision3VoiceBatchStore;
  bool get supportsQuestTranscript =>
      _store is ManagedRevision3QuestTranscriptStore;
  bool get supportsNpcGreeting => _store is ManagedRevision3NpcGreetingStore;
  bool get supportsDialogVoiceSlotRemoval =>
      _store is ManagedRevision3DialogVoiceSlotRemovalStore;
  bool get supportsNpcProfileEdit =>
      _store is ManagedRevision3NpcProfileEditStore;
  bool get supportsProjectHistory =>
      _store is ManagedRevision3ProjectHistoryStore;

  /// Fail closed after a higher-layer post-publication receipt mismatch. This
  /// can only remove authoring authority; regain it through verified in-session
  /// recovery or by closing and reopening the project.
  void markRequiresReopenAfterPublicationUncertainty() =>
      _core.markRequiresReopenAfterPublicationUncertainty();
  File get headFile => _core.headFile;

  static Future<ManagedRevision3AuthoringProjectSession> create({
    required Directory root,
    required ManagedRevision3AuthoringStore store,
    required String projectJson,
    AtomicByteReplacement? replacement,
  }) async => ManagedRevision3AuthoringProjectSession._(
    await _ManagedProjectSessionCore.create(
      root: root,
      store: _Revision3ManagedCheckpointStore(store),
      projectJson: projectJson,
      replacement: replacement,
    ),
    store,
  );

  static Future<ManagedRevision3AuthoringProjectSession> open({
    required Directory root,
    required ManagedRevision3AuthoringStore store,
    AtomicByteReplacement? replacement,
  }) async => ManagedRevision3AuthoringProjectSession._(
    await _ManagedProjectSessionCore.open(
      root: root,
      store: _Revision3ManagedCheckpointStore(store),
      replacement: replacement,
    ),
    store,
  );

  Future<void> save(String projectJson) => _core.save(projectJson);

  Future<T> deriveAndSave<T>(ManagedProjectDeriver<T> derive) =>
      _core.deriveAndSave(derive);

  /// Read one bounded newest-first lineage rooted at the exact fixed head.
  /// Physical CAS directories are never enumerated by this capability.
  Future<Revision3ProjectHistorySnapshot>
  readProjectHistoryV1() => _core.readExact<Revision3ProjectHistorySnapshot>(
    (basis) async {
      final store = _store;
      final projectId = basis.projectId;
      final projectRevision = basis.projectRevision;
      if (store is! ManagedRevision3ProjectHistoryStore ||
          projectId == null ||
          projectRevision == null) {
        throw UnsupportedError(
          'this managed revision-3 Store has no authenticated history capability',
        );
      }
      final historyStore = store as ManagedRevision3ProjectHistoryStore;
      final AuthoringRevision3ProjectHistoryResult result;
      try {
        result = await historyStore.listProjectHistoryV1(
          root: root.path,
          expectedHead: basis.head,
        );
      } on ModFfiException catch (error, stackTrace) {
        if (_revision3HistoryErrorNeedsCurrentReverification(error.code)) {
          await _core._reverifyExactCurrentAfterHistoryFailure(basis);
          Error.throwWithStackTrace(
            _ManagedRevision3HistoryFailureWithVerifiedCurrent(
              error,
              stackTrace,
            ),
            stackTrace,
          );
        }
        rethrow;
      }
      if (result.basisHead.canonicalJson != basis.head.canonicalJson ||
          result.projectId != projectId ||
          result.projectRevision != projectRevision) {
        throw const ManagedProjectVerificationException(
          'revision-3 history disagrees with its exact session basis',
        );
      }
      return Revision3ProjectHistorySnapshot(
        basisHead: result.basisHead,
        projectId: result.projectId,
        currentRevision: result.projectRevision,
        entries: [
          for (final entry in result.entries)
            Revision3ProjectHistoryEntry(
              head: entry.head,
              projectId: entry.projectId,
              projectRevision: entry.projectRevision,
              isCurrent: entry.current,
            ),
        ],
        historyTruncated: result.historyTruncated,
      );
    },
    operation: 'readProjectHistoryV1',
    handleReadError: _core._throwRevision3HistoryReadError,
  );

  /// Copy one authenticated ancestor into a fresh current+1 checkpoint.
  ///
  /// Native code is prepare-only. Publication remains in the managed
  /// full-reopen, exact-CAS, crash-recovery lane.
  Future<ManagedRevision3ProjectHistoryRestoreCheckpoint>
  prepareAndPublishProjectHistoryRestoreV1({
    required Revision3ProjectHistorySnapshot expectedHistory,
    required Revision3ProjectHistoryEntry target,
  }) =>
      _core._publishPreparedRevision3Checkpoint<
        ManagedRevision3ProjectHistoryRestoreCheckpoint
      >(
        operation: 'prepareAndPublishProjectHistoryRestoreV1',
        handlePrepareError: _core._throwRevision3HistoryRestorePrepareError,
        prepare: (basis) async {
          final store = _store;
          final projectId = basis.projectId;
          final projectRevision = basis.projectRevision;
          if (store is! ManagedRevision3ProjectHistoryStore ||
              projectId == null ||
              projectRevision == null) {
            throw UnsupportedError(
              'this managed revision-3 Store has no authenticated history restore capability',
            );
          }
          final historyStore = store as ManagedRevision3ProjectHistoryStore;
          final targetIsListed = expectedHistory.entries.any(
            (entry) =>
                !entry.isCurrent &&
                entry.head.canonicalJson == target.head.canonicalJson &&
                entry.projectRevision == target.projectRevision,
          );
          if (expectedHistory.basisHead.canonicalJson !=
                  basis.head.canonicalJson ||
              expectedHistory.projectId != projectId ||
              expectedHistory.currentRevision != projectRevision ||
              target.projectId != projectId ||
              target.isCurrent ||
              target.projectRevision >= projectRevision ||
              !targetIsListed) {
            throw const FormatException(
              'revision-3 history restore selection is stale or not authenticated by this view',
            );
          }
          final AuthoringRevision3ProjectHistoryRestorePreparation prepared;
          try {
            prepared = await historyStore.prepareProjectHistoryRestoreV1(
              root: root.path,
              expectedHead: basis.head,
              targetHead: target.head,
            );
          } on ModFfiException catch (error, stackTrace) {
            if (_revision3HistoryErrorNeedsCurrentReverification(error.code)) {
              await _core._reverifyExactCurrentAfterHistoryFailure(basis);
              Error.throwWithStackTrace(
                _ManagedRevision3HistoryFailureWithVerifiedCurrent(
                  error,
                  stackTrace,
                ),
                stackTrace,
              );
            }
            rethrow;
          }
          if (prepared.basisHead.canonicalJson != basis.head.canonicalJson ||
              prepared.directParentHead.canonicalJson !=
                  basis.head.canonicalJson ||
              prepared.restoredFromHead.canonicalJson !=
                  target.head.canonicalJson ||
              prepared.projectId != projectId ||
              prepared.previousProjectRevision != projectRevision ||
              prepared.revision != projectRevision + 1 ||
              prepared.restoredFromRevision != target.projectRevision) {
            throw const ManagedProjectVerificationException(
              'revision-3 history restore preparation disagrees with its exact basis or target',
            );
          }
          return _ManagedPreparedCheckpoint<
            ManagedRevision3ProjectHistoryRestoreCheckpoint
          >(
            head: prepared.head,
            projectJson: prepared.projectJson,
            value: ManagedRevision3ProjectHistoryRestoreCheckpoint(
              previousHead: basis.head,
              head: prepared.head,
              projectJson: prepared.projectJson,
              projectId: prepared.projectId,
              previousProjectRevision: prepared.previousProjectRevision,
              projectRevision: prepared.revision,
              restoredFromHead: prepared.restoredFromHead,
              restoredFromRevision: prepared.restoredFromRevision,
            ),
          );
        },
      );

  /// Prepare and publish one semantic revision-3 Quest Draft transaction.
  ///
  /// The request's head/project/revision binding is constructed only after this operation reaches
  /// the serialized session lane. Native code prepares immutable objects but cannot publish the
  /// fixed head. The session then requires an exact basis match, fully reopens the candidate,
  /// publishes it through the crash-recoverable byte-CAS lane, and fully reopens the published
  /// generation before returning. No game file is written and no build/runtime claim is created.
  Future<ManagedRevision3QuestDraftCheckpoint> prepareAndPublishQuestDraftV3({
    required String gameRoot,
    required String questId,
    required String scriptModuleId,
    required String displayName,
    required AuthoringRevision3QuestDraftIntentV3 intent,
  }) =>
      _core._publishPreparedRevision3Checkpoint<
        ManagedRevision3QuestDraftCheckpoint
      >(
        operation: 'prepareAndPublishQuestDraftV3',
        handlePrepareError: _core._throwRevision3QuestPrepareError,
        prepare: (basis) async {
          final projectId = basis.projectId;
          final projectRevision = basis.projectRevision;
          if (projectId == null || projectRevision == null) {
            throw const ManagedProjectVerificationException(
              'revision-3 Quest transaction has no exact project identity',
            );
          }
          final request = AuthoringRevision3QuestDraftRequestV3(
            expectedHead: basis.head,
            expectedProjectId: projectId,
            expectedRevision: projectRevision,
            questId: questId,
            scriptModuleId: scriptModuleId,
            displayName: displayName,
            intent: intent,
          );
          final prepared = await _store.prepareQuestDraftV3(
            root: root.path,
            gameRoot: gameRoot,
            currentProjectJson: basis.projectJson,
            questRequestJson: request.canonicalJson,
          );
          if (prepared.basisHead.canonicalJson != basis.head.canonicalJson ||
              prepared.projectId != projectId ||
              prepared.revision != projectRevision + 1 ||
              prepared.questId != request.questId ||
              prepared.scriptModuleId != request.scriptModuleId ||
              prepared.displayName != request.displayName ||
              prepared.moduleNamespace != request.intent.moduleNamespace ||
              prepared.technicalId != request.intent.technicalId ||
              prepared.textHelper != request.intent.textHelper ||
              prepared.title != request.intent.title ||
              prepared.description != request.intent.description ||
              prepared.objectiveTitle != request.intent.objectiveTitle ||
              !_sameOrderedStrings(
                prepared.additionalObjectiveTitles,
                request.intent.additionalObjectiveTitles,
              )) {
            throw const ManagedProjectVerificationException(
              'revision-3 Quest preparation disagrees with its exact session basis or request',
            );
          }
          return _ManagedPreparedCheckpoint<
            ManagedRevision3QuestDraftCheckpoint
          >(
            head: prepared.head,
            projectJson: prepared.projectJson,
            value: ManagedRevision3QuestDraftCheckpoint._(
              head: prepared.head,
              projectJson: prepared.projectJson,
              projectId: prepared.projectId,
              projectRevision: prepared.revision,
              questId: prepared.questId,
              scriptModuleId: prepared.scriptModuleId,
              artifactDeduplicated: prepared.artifactDeduplicated,
            ),
          );
        },
      );

  /// Edit only the visible outline of one exact-current Quest and regenerate
  /// its already-owned ScriptModule. The request is constructed inside the
  /// serialized lane; native collision context remains private. The same
  /// full-reopen, repair and exact byte-CAS publication lane is used as every
  /// other managed checkpoint edit.
  Future<ManagedRevision3QuestOutlineEditCheckpoint>
  prepareAndPublishQuestOutlineEditV1({
    required String questId,
    required int expectedQuestRevision,
    required String expectedModuleId,
    required int expectedModuleRevision,
    required String displayName,
    required String title,
    required List<String> objectiveTitles,
  }) =>
      _core._publishPreparedRevision3Checkpoint<
        ManagedRevision3QuestOutlineEditCheckpoint
      >(
        operation: 'prepareAndPublishQuestOutlineEditV1',
        handlePrepareError: _core._throwRevision3QuestOutlinePrepareError,
        prepare: (basis) async {
          final projectId = basis.projectId;
          final projectRevision = basis.projectRevision;
          if (projectId == null || projectRevision == null) {
            throw const ManagedProjectVerificationException(
              'revision-3 Quest outline edit has no exact project identity',
            );
          }
          final request =
              AuthoringRevision3QuestOutlineEditRequestV1.forProject(
                expectedHead: basis.head,
                currentProjectJson: basis.projectJson,
                questId: questId,
                expectedQuestRevision: expectedQuestRevision,
                displayName: displayName,
                title: title,
                objectiveTitles: objectiveTitles,
              );
          if (request.moduleId != expectedModuleId ||
              request.expectedModuleRevision != expectedModuleRevision) {
            throw const FormatException(
              'revision-3 Quest outline edit does not bind the selected Quest module',
            );
          }
          final prepared = await _store.prepareQuestOutlineEditV1(
            root: root.path,
            currentProjectJson: basis.projectJson,
            request: request,
          );
          if (prepared.basisHead.canonicalJson != basis.head.canonicalJson ||
              prepared.projectId != projectId ||
              prepared.revision != projectRevision + 1 ||
              prepared.questId != request.questId ||
              prepared.moduleId != request.moduleId ||
              prepared.questRevision != request.expectedQuestRevision + 1 ||
              prepared.moduleRevision != request.expectedModuleRevision + 1) {
            throw const ManagedProjectVerificationException(
              'revision-3 Quest outline preparation disagrees with its exact session basis or request',
            );
          }
          return _ManagedPreparedCheckpoint<
            ManagedRevision3QuestOutlineEditCheckpoint
          >(
            head: prepared.head,
            projectJson: prepared.projectJson,
            value: ManagedRevision3QuestOutlineEditCheckpoint._(
              head: prepared.head,
              projectJson: prepared.projectJson,
              projectId: prepared.projectId,
              projectRevision: prepared.revision,
              questId: prepared.questId,
              moduleId: prepared.moduleId,
              questRevision: prepared.questRevision,
              moduleRevision: prepared.moduleRevision,
            ),
          );
        },
      );

  /// Stable-slot-aware outline edit for an exact semantic Quest. Objective
  /// titles and order may change, while the active slot set and transition
  /// graph remain bound to the exact transition-plan seal.
  Future<ManagedRevision3QuestOutlineEditCheckpoint>
  prepareAndPublishQuestOutlineEditV2({
    required String questId,
    required int expectedQuestRevision,
    required String expectedModuleId,
    required int expectedModuleRevision,
    required AuthoringDraftContentSeal expectedTransitionPlanSeal,
    required String displayName,
    required String title,
    required List<int> objectiveSlots,
    required List<String> objectiveTitles,
  }) =>
      _core._publishPreparedRevision3Checkpoint<
        ManagedRevision3QuestOutlineEditCheckpoint
      >(
        operation: 'prepareAndPublishQuestOutlineEditV2',
        handlePrepareError: _core._throwRevision3QuestOutlinePrepareError,
        prepare: (basis) async {
          final projectId = basis.projectId;
          final projectRevision = basis.projectRevision;
          if (projectId == null || projectRevision == null) {
            throw const ManagedProjectVerificationException(
              'revision-3 Quest outline-v2 edit has no exact project identity',
            );
          }
          if (objectiveSlots.length != objectiveTitles.length) {
            throw const FormatException(
              'revision-3 Quest outline-v2 slots and titles disagree',
            );
          }
          final request =
              AuthoringRevision3QuestOutlineEditRequestV2.forProject(
                expectedHead: basis.head,
                currentProjectJson: basis.projectJson,
                questId: questId,
                expectedQuestRevision: expectedQuestRevision,
                expectedModuleId: expectedModuleId,
                expectedModuleRevision: expectedModuleRevision,
                expectedTransitionPlanSeal: expectedTransitionPlanSeal,
                displayName: displayName,
                questTitle: title,
                objectives: [
                  for (var index = 0; index < objectiveSlots.length; index++)
                    AuthoringRevision3QuestOutlineObjectiveEditV2(
                      slot: objectiveSlots[index],
                      title: objectiveTitles[index],
                    ),
                ],
              );
          final prepared = await _store.prepareQuestOutlineEditV2(
            root: root.path,
            currentProjectJson: basis.projectJson,
            request: request,
          );
          if (prepared.basisHead.canonicalJson != basis.head.canonicalJson ||
              prepared.projectId != projectId ||
              prepared.revision != projectRevision + 1 ||
              prepared.questId != request.questId ||
              prepared.moduleId != request.moduleId ||
              prepared.questRevision != request.expectedQuestRevision + 1 ||
              prepared.moduleRevision != request.expectedModuleRevision + 1 ||
              prepared.buildStatus !=
                  AuthoringRevision3QuestOutlineBuildStatus.blocked ||
              prepared.runtimeStatus !=
                  AuthoringRevision3QuestOutlineRuntimeStatus
                      .runtimeUnqualified ||
              prepared.publicationStatus !=
                  AuthoringRevision3QuestOutlinePublicationStatus
                      .notSupported) {
            throw const ManagedProjectVerificationException(
              'revision-3 Quest outline-v2 preparation disagrees with its exact session basis or request',
            );
          }
          return _ManagedPreparedCheckpoint<
            ManagedRevision3QuestOutlineEditCheckpoint
          >(
            head: prepared.head,
            projectJson: prepared.projectJson,
            value: ManagedRevision3QuestOutlineEditCheckpoint._(
              head: prepared.head,
              projectJson: prepared.projectJson,
              projectId: prepared.projectId,
              projectRevision: prepared.revision,
              questId: prepared.questId,
              moduleId: prepared.moduleId,
              questRevision: prepared.questRevision,
              moduleRevision: prepared.moduleRevision,
            ),
          );
        },
      );

  Future<ManagedRevision3QuestTranscriptCheckpoint>
  prepareAndPublishQuestTranscriptReplaceV1({
    required Revision3QuestTranscriptReplaceTechnicalPlan plan,
  }) => _prepareAndPublishQuestTranscriptV1(
    operation: 'prepareAndPublishQuestTranscriptReplaceV1',
    questId: plan.questId,
    expectedQuestRevision: plan.expectedQuestRevision,
    expectedModuleId: plan.expectedModuleId,
    expectedModuleRevision: plan.expectedModuleRevision,
    intentForBasis: (basis) => AuthoringRevision3QuestTranscriptReplaceIntentV1(
      bindings: plan.bindings,
    ),
  );

  Future<ManagedRevision3QuestTranscriptCheckpoint>
  prepareAndPublishQuestTranscriptCreateV1({
    required Revision3QuestTranscriptCreateTechnicalPlan plan,
  }) => _prepareAndPublishQuestTranscriptV1(
    operation: 'prepareAndPublishQuestTranscriptCreateV1',
    questId: plan.questId,
    expectedQuestRevision: plan.expectedQuestRevision,
    expectedModuleId: plan.expectedModuleId,
    expectedModuleRevision: plan.expectedModuleRevision,
    intentForBasis: (basis) {
      final line = plan.line;
      return AuthoringRevision3QuestTranscriptCreateAndInsertIntentV1(
        index: plan.index,
        objectiveSlot: plan.objectiveSlot,
        line: AuthoringRevision3DialogLineEntryRequestV1.forProject(
          expectedHead: basis.head,
          currentProjectJson: basis.projectJson,
          lineId: line.lineId,
          lineDisplayName: line.lineDisplayName,
          lineAuthoredIdentity: line.lineAuthoredIdentity,
          speakerHint: line.speakerHint,
          localization: line.localization,
          voiceSlot: line.voiceSlot,
        ),
      );
    },
  );

  Future<ManagedRevision3QuestTranscriptCheckpoint>
  _prepareAndPublishQuestTranscriptV1({
    required String operation,
    required String questId,
    required int expectedQuestRevision,
    required String expectedModuleId,
    required int expectedModuleRevision,
    required AuthoringRevision3QuestTranscriptIntentV1 Function(
      _ManagedOpenedCheckpoint basis,
    )
    intentForBasis,
  }) {
    final store = _store;
    if (store is! ManagedRevision3QuestTranscriptStore) {
      return Future<ManagedRevision3QuestTranscriptCheckpoint>.error(
        UnsupportedError(
          'this managed revision-3 Store has no Quest transcript capability',
        ),
      );
    }
    final transcriptStore = store as ManagedRevision3QuestTranscriptStore;
    return _core._publishPreparedRevision3Checkpoint<
      ManagedRevision3QuestTranscriptCheckpoint
    >(
      operation: operation,
      handlePrepareError: _core._throwRevision3QuestTranscriptPrepareError,
      prepare: (basis) async {
        final projectId = basis.projectId;
        final projectRevision = basis.projectRevision;
        if (projectId == null || projectRevision == null) {
          throw const ManagedProjectVerificationException(
            'revision-3 Quest transcript edit has no exact project identity',
          );
        }
        final intent = intentForBasis(basis);
        final request = AuthoringRevision3QuestTranscriptRequestV1.forProject(
          expectedHead: basis.head,
          currentProjectJson: basis.projectJson,
          questId: questId,
          expectedQuestRevision: expectedQuestRevision,
          intent: intent,
        );
        if (request.moduleId != expectedModuleId ||
            request.expectedModuleRevision != expectedModuleRevision) {
          throw const FormatException(
            'revision-3 Quest transcript edit does not bind the selected Quest module',
          );
        }
        final prepared = await transcriptStore.prepareQuestTranscriptV1(
          root: root.path,
          currentProjectJson: basis.projectJson,
          request: request,
        );
        final expectedCreatedLine =
            intent is AuthoringRevision3QuestTranscriptCreateAndInsertIntentV1
            ? intent.line.lineId
            : null;
        final expectedCreatedLocalization =
            intent is AuthoringRevision3QuestTranscriptCreateAndInsertIntentV1
            ? intent.line.localization.localizationId
            : null;
        final expectedCreatedVoice =
            intent is AuthoringRevision3QuestTranscriptCreateAndInsertIntentV1
            ? intent.line.voiceSlot?.slotId
            : null;
        final expectedAction =
            intent is AuthoringRevision3QuestTranscriptCreateAndInsertIntentV1
            ? (intent.line.localization
                      is AuthoringRevision3DialogLocalizationCreateIntentV1
                  ? AuthoringRevision3DialogLocalizationAction.created
                  : AuthoringRevision3DialogLocalizationAction.reusedExact)
            : null;
        if (prepared.basisHead.canonicalJson != basis.head.canonicalJson ||
            prepared.projectId != projectId ||
            prepared.revision != projectRevision + 1 ||
            prepared.questId != request.questId ||
            prepared.questRevision != request.expectedQuestRevision + 1 ||
            prepared.moduleId != request.moduleId ||
            prepared.moduleRevision != request.expectedModuleRevision ||
            prepared.mode != intent.mode ||
            prepared.createdLineId != expectedCreatedLine ||
            prepared.createdLocalizationId != expectedCreatedLocalization ||
            prepared.createdVoiceSlotId != expectedCreatedVoice ||
            prepared.localizationAction != expectedAction ||
            prepared.buildStatus !=
                AuthoringRevision3DialogBuildStatus.blocked ||
            prepared.runtimeStatus !=
                AuthoringRevision3DialogRuntimeStatus.runtimeUnqualified ||
            prepared.topicAuthority !=
                AuthoringRevision3DialogTopicAuthority.notGranted ||
            prepared.publicationStatus !=
                AuthoringRevision3DialogPublicationStatus.notSupported) {
          throw const ManagedProjectVerificationException(
            'revision-3 Quest transcript preparation disagrees with its exact session basis or intent',
          );
        }
        return _ManagedPreparedCheckpoint<
          ManagedRevision3QuestTranscriptCheckpoint
        >(
          head: prepared.head,
          projectJson: prepared.projectJson,
          value: ManagedRevision3QuestTranscriptCheckpoint._(
            head: prepared.head,
            projectJson: prepared.projectJson,
            projectId: prepared.projectId,
            projectRevision: prepared.revision,
            questId: prepared.questId,
            questRevision: prepared.questRevision,
            moduleId: prepared.moduleId,
            moduleRevision: prepared.moduleRevision,
            mode: prepared.mode,
            transcriptCount: prepared.transcriptCount,
            createdLineId: prepared.createdLineId,
            createdLocalizationId: prepared.createdLocalizationId,
            createdVoiceSlotId: prepared.createdVoiceSlotId,
            localizationAction: prepared.localizationAction,
          ),
        );
      },
    );
  }

  Future<ManagedRevision3NpcGreetingCheckpoint>
  prepareAndPublishNpcGreetingReplaceV1({
    required Revision3NpcGreetingReplaceTechnicalPlan plan,
  }) => _prepareAndPublishNpcGreetingV1(
    operation: 'prepareAndPublishNpcGreetingReplaceV1',
    npcId: plan.npcId,
    expectedNpcRevision: plan.expectedNpcRevision,
    expectedModuleId: plan.expectedModuleId,
    expectedModuleRevision: plan.expectedModuleRevision,
    expectedGreetingCount: null,
    intentForBasis: (basis) =>
        AuthoringRevision3NpcGreetingReplaceIntentV1(bindings: plan.bindings),
  );

  Future<ManagedRevision3NpcGreetingCheckpoint>
  prepareAndPublishNpcGreetingCreateV1({
    required Revision3NpcGreetingCreateTechnicalPlan plan,
  }) => _prepareAndPublishNpcGreetingV1(
    operation: 'prepareAndPublishNpcGreetingCreateV1',
    npcId: plan.npcId,
    expectedNpcRevision: plan.expectedNpcRevision,
    expectedModuleId: plan.expectedModuleId,
    expectedModuleRevision: plan.expectedModuleRevision,
    expectedGreetingCount: plan.expectedGreetingCount,
    intentForBasis: (basis) {
      final line = plan.line;
      return AuthoringRevision3NpcGreetingCreateAndInsertIntentV1(
        index: plan.index,
        line: AuthoringRevision3DialogLineEntryRequestV1.forProject(
          expectedHead: basis.head,
          currentProjectJson: basis.projectJson,
          lineId: line.lineId,
          lineDisplayName: line.lineDisplayName,
          lineAuthoredIdentity: line.lineAuthoredIdentity,
          speakerHint: line.speakerHint,
          localization: line.localization,
          voiceSlot: line.voiceSlot,
        ),
      );
    },
  );

  Future<ManagedRevision3NpcGreetingCheckpoint>
  _prepareAndPublishNpcGreetingV1({
    required String operation,
    required String npcId,
    required int expectedNpcRevision,
    required String expectedModuleId,
    required int expectedModuleRevision,
    required int? expectedGreetingCount,
    required AuthoringRevision3NpcGreetingIntentV1 Function(
      _ManagedOpenedCheckpoint basis,
    )
    intentForBasis,
  }) {
    final store = _store;
    if (store is! ManagedRevision3NpcGreetingStore) {
      return Future<ManagedRevision3NpcGreetingCheckpoint>.error(
        UnsupportedError(
          'this managed revision-3 Store has no NPC greeting capability',
        ),
      );
    }
    final greetingStore = store as ManagedRevision3NpcGreetingStore;
    return _core._publishPreparedRevision3Checkpoint<
      ManagedRevision3NpcGreetingCheckpoint
    >(
      operation: operation,
      handlePrepareError: _core._throwRevision3NpcGreetingPrepareError,
      prepare: (basis) async {
        final projectId = basis.projectId;
        final projectRevision = basis.projectRevision;
        if (projectId == null || projectRevision == null) {
          throw const ManagedProjectVerificationException(
            'revision-3 NPC greeting edit has no exact project identity',
          );
        }
        final intent = intentForBasis(basis);
        final request = AuthoringRevision3NpcGreetingRequestV1.forProject(
          expectedHead: basis.head,
          currentProjectJson: basis.projectJson,
          npcId: npcId,
          expectedNpcRevision: expectedNpcRevision,
          intent: intent,
        );
        if (request.moduleId != expectedModuleId ||
            request.expectedModuleRevision != expectedModuleRevision ||
            (expectedGreetingCount != null &&
                request.expectedGreetingCount != expectedGreetingCount)) {
          throw const FormatException(
            'revision-3 NPC greeting edit does not bind the selected NPC checkpoint',
          );
        }
        final prepared = await greetingStore.prepareNpcGreetingV1(
          root: root.path,
          currentProjectJson: basis.projectJson,
          request: request,
        );
        final expectedCreatedLine =
            intent is AuthoringRevision3NpcGreetingCreateAndInsertIntentV1
            ? intent.line.lineId
            : null;
        final expectedCreatedLocalization =
            intent is AuthoringRevision3NpcGreetingCreateAndInsertIntentV1
            ? intent.line.localization.localizationId
            : null;
        final expectedCreatedVoice =
            intent is AuthoringRevision3NpcGreetingCreateAndInsertIntentV1
            ? intent.line.voiceSlot?.slotId
            : null;
        final expectedAction =
            intent is AuthoringRevision3NpcGreetingCreateAndInsertIntentV1
            ? (intent.line.localization
                      is AuthoringRevision3DialogLocalizationCreateIntentV1
                  ? AuthoringRevision3DialogLocalizationAction.created
                  : AuthoringRevision3DialogLocalizationAction.reusedExact)
            : null;
        final expectedCount = switch (intent) {
          AuthoringRevision3NpcGreetingReplaceIntentV1(:final bindings) =>
            bindings.length,
          AuthoringRevision3NpcGreetingCreateAndInsertIntentV1() =>
            request.expectedGreetingCount + 1,
        };
        if (prepared.basisHead.canonicalJson != basis.head.canonicalJson ||
            prepared.projectId != projectId ||
            prepared.revision != projectRevision + 1 ||
            prepared.npcId != request.npcId ||
            prepared.npcRevision != request.expectedNpcRevision + 1 ||
            prepared.moduleId != request.moduleId ||
            prepared.moduleRevision != request.expectedModuleRevision ||
            prepared.mode != intent.mode ||
            prepared.greetingCount != expectedCount ||
            prepared.createdLineId != expectedCreatedLine ||
            prepared.createdLocalizationId != expectedCreatedLocalization ||
            prepared.createdVoiceSlotId != expectedCreatedVoice ||
            prepared.localizationAction != expectedAction ||
            prepared.buildStatus !=
                AuthoringRevision3DialogBuildStatus.blocked ||
            prepared.runtimeStatus !=
                AuthoringRevision3DialogRuntimeStatus.runtimeUnqualified ||
            prepared.topicAuthority !=
                AuthoringRevision3DialogTopicAuthority.notGranted ||
            prepared.publicationStatus !=
                AuthoringRevision3DialogPublicationStatus.notSupported) {
          throw const ManagedProjectVerificationException(
            'revision-3 NPC greeting preparation disagrees with its exact session basis or intent',
          );
        }
        return _ManagedPreparedCheckpoint<
          ManagedRevision3NpcGreetingCheckpoint
        >(
          head: prepared.head,
          projectJson: prepared.projectJson,
          value: ManagedRevision3NpcGreetingCheckpoint._(
            head: prepared.head,
            projectJson: prepared.projectJson,
            projectId: prepared.projectId,
            projectRevision: prepared.revision,
            npcId: prepared.npcId,
            npcRevision: prepared.npcRevision,
            moduleId: prepared.moduleId,
            moduleRevision: prepared.moduleRevision,
            mode: prepared.mode,
            greetingCount: prepared.greetingCount,
            createdLineId: prepared.createdLineId,
            createdLocalizationId: prepared.createdLocalizationId,
            createdVoiceSlotId: prepared.createdVoiceSlotId,
            localizationAction: prepared.localizationAction,
          ),
        );
      },
    );
  }

  /// Edit one exact-current Quest transition plan without consulting a game
  /// installation. The effective legacy/V4 seed and its seal are derived from
  /// the canonical project only after entering the serialized session lane.
  /// Native code returns an unpublished candidate; the common managed
  /// checkpoint path performs both full reopens and the fixed-head CAS.
  Future<ManagedRevision3QuestTransitionsEditCheckpoint>
  prepareAndPublishQuestTransitionsEditV1({
    required String questId,
    required int expectedQuestRevision,
    required String expectedModuleId,
    required int expectedModuleRevision,
    required AuthoringDraftContentSeal expectedTransitionPlanSeal,
    required AuthoringRevision3QuestTransitionPlanV1 transitionPlan,
  }) =>
      _core._publishPreparedRevision3Checkpoint<
        ManagedRevision3QuestTransitionsEditCheckpoint
      >(
        operation: 'prepareAndPublishQuestTransitionsEditV1',
        handlePrepareError: _core._throwRevision3QuestTransitionsPrepareError,
        prepare: (basis) async {
          final projectId = basis.projectId;
          final projectRevision = basis.projectRevision;
          if (projectId == null || projectRevision == null) {
            throw const ManagedProjectVerificationException(
              'revision-3 Quest transitions edit has no exact project identity',
            );
          }
          final request =
              AuthoringRevision3QuestTransitionsEditRequestV1.forProject(
                expectedHead: basis.head,
                currentProjectJson: basis.projectJson,
                questId: questId,
                expectedQuestRevision: expectedQuestRevision,
                transitionPlan: transitionPlan,
              );
          if (request.moduleId != expectedModuleId ||
              request.expectedModuleRevision != expectedModuleRevision ||
              !_sameDraftContentSeal(
                request.expectedTransitionPlanSeal,
                expectedTransitionPlanSeal,
              )) {
            throw const FormatException(
              'revision-3 Quest transitions edit does not bind the selected Quest plan/module',
            );
          }
          final prepared = await _store.prepareQuestTransitionsEditV1(
            root: root.path,
            currentProjectJson: basis.projectJson,
            request: request,
          );
          if (prepared.basisHead.canonicalJson != basis.head.canonicalJson ||
              prepared.projectId != projectId ||
              prepared.revision != projectRevision + 1 ||
              prepared.questId != request.questId ||
              prepared.moduleId != request.moduleId ||
              prepared.questRevision != request.expectedQuestRevision + 1 ||
              prepared.moduleRevision != request.expectedModuleRevision + 1 ||
              prepared.previousGeneratorVersion !=
                  request.previousGeneratorVersion ||
              prepared.upgradedFromLegacy != request.upgradesLegacy ||
              !_sameDraftContentSeal(
                prepared.transitionPlanSeal,
                request.transitionPlan.contentSeal,
              ) ||
              prepared.buildStatus !=
                  AuthoringRevision3QuestTransitionsBuildStatus.blocked ||
              prepared.runtimeStatus !=
                  AuthoringRevision3QuestTransitionsRuntimeStatus
                      .runtimeUnqualified ||
              prepared.publicationStatus !=
                  AuthoringRevision3QuestTransitionsPublicationStatus
                      .notSupported) {
            throw const ManagedProjectVerificationException(
              'revision-3 Quest transitions preparation disagrees with its exact session basis or request',
            );
          }
          return _ManagedPreparedCheckpoint<
            ManagedRevision3QuestTransitionsEditCheckpoint
          >(
            head: prepared.head,
            projectJson: prepared.projectJson,
            value: ManagedRevision3QuestTransitionsEditCheckpoint._(
              head: prepared.head,
              projectJson: prepared.projectJson,
              projectId: prepared.projectId,
              projectRevision: prepared.revision,
              questId: prepared.questId,
              moduleId: prepared.moduleId,
              questRevision: prepared.questRevision,
              moduleRevision: prepared.moduleRevision,
              previousGeneratorVersion: prepared.previousGeneratorVersion,
              upgradedFromLegacy: prepared.upgradedFromLegacy,
              transitionPlanSeal: prepared.transitionPlanSeal,
              buildStatus: prepared.buildStatus,
              runtimeStatus: prepared.runtimeStatus,
              publicationStatus: prepared.publicationStatus,
            ),
          );
        },
      );

  /// Edit the description/family/giver context of one exact-current Quest.
  /// Fresh catalog authority remains native; this session constructs the
  /// request only inside its serialized basis and publishes through the common
  /// full-reopen, repair and exact byte-CAS lane.
  Future<ManagedRevision3QuestContextEditCheckpoint>
  prepareAndPublishQuestContextEditV1({
    required String gameRoot,
    required String questId,
    required int expectedQuestRevision,
    required String expectedModuleId,
    required int expectedModuleRevision,
    required AuthoringDraftContentSeal expectedStoryCatalogSeal,
    required String description,
    required String parentCatalogId,
    required String giverCatalogId,
    required String expectedParentRuntimeClass,
    required String expectedParentCatalogLayer,
    required String expectedParentAuthoringSelector,
    required AuthoringDraftContentSeal expectedParentSourceSeal,
    required String expectedGiverRuntimeUniqueName,
    required String expectedGiverCatalogLayer,
    required String expectedGiverAuthoringSelector,
    required AuthoringDraftContentSeal expectedGiverSourceSeal,
  }) =>
      _core._publishPreparedRevision3Checkpoint<
        ManagedRevision3QuestContextEditCheckpoint
      >(
        operation: 'prepareAndPublishQuestContextEditV1',
        handlePrepareError: _core._throwRevision3QuestContextPrepareError,
        prepare: (basis) async {
          final projectId = basis.projectId;
          final projectRevision = basis.projectRevision;
          if (projectId == null || projectRevision == null) {
            throw const ManagedProjectVerificationException(
              'revision-3 Quest context edit has no exact project identity',
            );
          }
          final request =
              AuthoringRevision3QuestContextEditRequestV1.forProject(
                expectedHead: basis.head,
                currentProjectJson: basis.projectJson,
                expectedStoryCatalogSeal: expectedStoryCatalogSeal,
                questId: questId,
                expectedQuestRevision: expectedQuestRevision,
                description: description,
                parentCatalogId: parentCatalogId,
                giverCatalogId: giverCatalogId,
                expectedParentRuntimeClass: expectedParentRuntimeClass,
                expectedParentCatalogLayer: expectedParentCatalogLayer,
                expectedParentAuthoringSelector:
                    expectedParentAuthoringSelector,
                expectedParentSourceSeal: expectedParentSourceSeal,
                expectedGiverRuntimeUniqueName: expectedGiverRuntimeUniqueName,
                expectedGiverCatalogLayer: expectedGiverCatalogLayer,
                expectedGiverAuthoringSelector: expectedGiverAuthoringSelector,
                expectedGiverSourceSeal: expectedGiverSourceSeal,
              );
          if (request.moduleId != expectedModuleId ||
              request.expectedModuleRevision != expectedModuleRevision) {
            throw const FormatException(
              'revision-3 Quest context edit does not bind the selected Quest module',
            );
          }
          final prepared = await _store.prepareQuestContextEditV1(
            root: root.path,
            gameRoot: gameRoot,
            currentProjectJson: basis.projectJson,
            request: request,
          );
          if (prepared.basisHead.canonicalJson != basis.head.canonicalJson ||
              prepared.projectId != projectId ||
              prepared.revision != projectRevision + 1 ||
              prepared.questId != request.questId ||
              prepared.moduleId != request.moduleId ||
              prepared.questRevision != request.expectedQuestRevision + 1 ||
              prepared.moduleRevision != request.expectedModuleRevision + 1 ||
              prepared.parentCatalogId != request.parentCatalogId ||
              prepared.giverCatalogId != request.giverCatalogId ||
              prepared.parentRuntimeClass != expectedParentRuntimeClass ||
              prepared.parentCatalogLayer != expectedParentCatalogLayer ||
              prepared.parentAuthoringSelector !=
                  expectedParentAuthoringSelector ||
              !_sameDraftContentSeal(
                prepared.parentSourceSeal,
                expectedParentSourceSeal,
              ) ||
              prepared.giverRuntimeUniqueName !=
                  expectedGiverRuntimeUniqueName ||
              prepared.giverCatalogLayer != expectedGiverCatalogLayer ||
              prepared.giverAuthoringSelector !=
                  expectedGiverAuthoringSelector ||
              !_sameDraftContentSeal(
                prepared.giverSourceSeal,
                expectedGiverSourceSeal,
              )) {
            throw const ManagedProjectVerificationException(
              'revision-3 Quest context preparation disagrees with its exact session basis or request',
            );
          }
          return _ManagedPreparedCheckpoint<
            ManagedRevision3QuestContextEditCheckpoint
          >(
            head: prepared.head,
            projectJson: prepared.projectJson,
            value: ManagedRevision3QuestContextEditCheckpoint._(
              head: prepared.head,
              projectJson: prepared.projectJson,
              projectId: prepared.projectId,
              projectRevision: prepared.revision,
              questId: prepared.questId,
              moduleId: prepared.moduleId,
              questRevision: prepared.questRevision,
              moduleRevision: prepared.moduleRevision,
              parentRuntimeClass: prepared.parentRuntimeClass,
              giverRuntimeUniqueName: prepared.giverRuntimeUniqueName,
            ),
          );
        },
      );

  /// Prepare and publish one offline-only revision-3 NPC Draft/module pair.
  ///
  /// The request's project ID, revision, target, and head are derived only after entering the
  /// serialized session lane. Native preparation may install immutable CAS objects but cannot
  /// replace the fixed head. This session independently checks the complete response binding,
  /// fully reopens the candidate, publishes through the crash-recoverable exact byte-CAS lane,
  /// and fully reopens the published generation before returning.
  Future<ManagedRevision3NpcDraftCheckpoint> prepareAndPublishNpcDraftV1({
    required String gameRoot,
    required String npcId,
    required String scriptModuleId,
    required String displayName,
    required AuthoringRevision3NpcDraftIntentV1 intent,
  }) => _core
      ._publishPreparedRevision3Checkpoint<ManagedRevision3NpcDraftCheckpoint>(
        operation: 'prepareAndPublishNpcDraftV1',
        handlePrepareError: _core._throwRevision3NpcPrepareError,
        prepare: (basis) async {
          final projectId = basis.projectId;
          final projectRevision = basis.projectRevision;
          if (projectId == null || projectRevision == null) {
            throw const ManagedProjectVerificationException(
              'revision-3 NPC transaction has no exact project identity',
            );
          }
          final request = AuthoringRevision3NpcDraftRequestV1.forProject(
            expectedHead: basis.head,
            currentProjectJson: basis.projectJson,
            npcId: npcId,
            scriptModuleId: scriptModuleId,
            displayName: displayName,
            intent: intent,
          );
          final prepared = await _store.prepareNpcDraftV1(
            root: root.path,
            gameRoot: gameRoot,
            currentProjectJson: basis.projectJson,
            request: request,
          );
          if (prepared.basisHead.canonicalJson != basis.head.canonicalJson ||
              prepared.projectId != projectId ||
              prepared.revision != projectRevision + 1 ||
              prepared.npcId != request.npcId ||
              prepared.scriptModuleId != request.scriptModuleId ||
              prepared.displayName != request.displayName ||
              prepared.moduleNamespace != request.intent.moduleNamespace ||
              prepared.uniqueName != request.intent.uniqueName ||
              prepared.parentCatalogId != request.intent.parentCatalogId) {
            throw const ManagedProjectVerificationException(
              'revision-3 NPC preparation disagrees with its exact session basis or request',
            );
          }
          return _ManagedPreparedCheckpoint<ManagedRevision3NpcDraftCheckpoint>(
            head: prepared.head,
            projectJson: prepared.projectJson,
            value: ManagedRevision3NpcDraftCheckpoint._(
              head: prepared.head,
              projectJson: prepared.projectJson,
              projectId: prepared.projectId,
              projectRevision: prepared.revision,
              npcId: prepared.npcId,
              scriptModuleId: prepared.scriptModuleId,
              displayName: prepared.displayName,
              moduleNamespace: prepared.moduleNamespace,
              uniqueName: prepared.uniqueName,
              parentCatalogId: prepared.parentCatalogId,
            ),
          );
        },
      );

  /// Derive one exact NPC profile seed locally from the serialized canonical
  /// project basis. No native command, object preparation, or publication is
  /// performed.
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
  }) => _core.readExact<AuthoringRevision3NpcProfileEditSeed>(
    (basis) async => AuthoringRevision3NpcProfileEditSeed.forProject(
      head: basis.head,
      currentProjectJson: basis.projectJson,
      npcId: npcId,
      expectedNpcRevision: expectedNpcRevision,
      expectedScriptModuleId: expectedScriptModuleId,
      expectedScriptModuleRevision: expectedScriptModuleRevision,
      expectedUniqueName: expectedUniqueName,
      expectedModuleNamespace: expectedModuleNamespace,
      expectedParentCharacterDefinition: expectedParentCharacterDefinition,
      expectedParentAiAgentConfig: expectedParentAiAgentConfig,
      expectedParentSpawnDefinition: expectedParentSpawnDefinition,
    ),
    operation: 'readNpcProfileEditSeedV1',
    handleReadError: _core._throwRevision3NpcProfileEditSeedError,
  );

  /// Prepare and publish one bounded existing-NPC display-name/archetype edit.
  Future<ManagedRevision3NpcProfileEditCheckpoint>
  prepareAndPublishNpcProfileEditV1({
    required String gameRoot,
    required AuthoringRevision3NpcProfileEditSeed seed,
    required AuthoringDraftContentSeal expectedStoryCatalogSeal,
    required AuthoringDraftContentSeal expectedNpcCatalogSeal,
    required String expectedParentCatalogId,
    required AuthoringRevision3NpcProfileParentTripleExpectation
    expectedCurrentParentTriple,
    required String displayName,
    required String parentCatalogId,
    required AuthoringRevision3NpcProfileParentTripleExpectation
    expectedParentTriple,
    required bool expectedArchetypeChanged,
    required bool expectedModuleRegenerated,
  }) {
    final editStore = _store;
    if (editStore is! ManagedRevision3NpcProfileEditStore) {
      return Future<ManagedRevision3NpcProfileEditCheckpoint>.error(
        UnsupportedError(
          'this managed revision-3 Store has no NPC profile edit capability',
        ),
      );
    }
    final capability = editStore as ManagedRevision3NpcProfileEditStore;
    return _core._publishPreparedRevision3Checkpoint<
      ManagedRevision3NpcProfileEditCheckpoint
    >(
      operation: 'prepareAndPublishNpcProfileEditV1',
      handlePrepareError: _core._throwRevision3NpcProfileEditPrepareError,
      prepare: (basis) async {
        final projectId = basis.projectId;
        final projectRevision = basis.projectRevision;
        if (projectId == null || projectRevision == null) {
          throw const ManagedProjectVerificationException(
            'revision-3 NPC profile edit has no exact project identity',
          );
        }
        final request = AuthoringRevision3NpcProfileEditRequestV1.forProject(
          expectedHead: basis.head,
          currentProjectJson: basis.projectJson,
          seed: seed,
          expectedStoryCatalogSeal: expectedStoryCatalogSeal,
          expectedNpcCatalogSeal: expectedNpcCatalogSeal,
          expectedParentCatalogId: expectedParentCatalogId,
          expectedCurrentParentTriple: expectedCurrentParentTriple,
          displayName: displayName,
          parentCatalogId: parentCatalogId,
          expectedParentTriple: expectedParentTriple,
          expectedArchetypeChanged: expectedArchetypeChanged,
          expectedModuleRegenerated: expectedModuleRegenerated,
        );
        final prepared = await capability.prepareNpcProfileEditV1(
          root: root.path,
          gameRoot: gameRoot,
          currentProjectJson: basis.projectJson,
          request: request,
        );
        if (prepared.basisHead.canonicalJson != basis.head.canonicalJson ||
            prepared.projectId != projectId ||
            prepared.revision != projectRevision + 1 ||
            prepared.npcId != request.npcId ||
            prepared.npcRevision != request.expectedNpcRevision + 1 ||
            prepared.scriptModuleId != request.scriptModuleId ||
            prepared.scriptModuleRevision !=
                request.expectedScriptModuleRevision +
                    (request.expectsModuleRegenerated ? 1 : 0) ||
            prepared.displayName != request.displayName ||
            prepared.previousParentCatalogId !=
                request.expectedParentCatalogId ||
            prepared.parentCatalogId != request.parentCatalogId ||
            prepared.nameChanged != request.expectsNameChanged ||
            prepared.archetypeChanged != request.expectsArchetypeChanged ||
            prepared.moduleRegenerated != request.expectsModuleRegenerated) {
          throw const ManagedProjectVerificationException(
            'revision-3 NPC profile preparation disagrees with its exact session basis or request',
          );
        }
        return _ManagedPreparedCheckpoint<
          ManagedRevision3NpcProfileEditCheckpoint
        >(
          head: prepared.head,
          projectJson: prepared.projectJson,
          value: ManagedRevision3NpcProfileEditCheckpoint._(
            head: prepared.head,
            projectJson: prepared.projectJson,
            projectId: prepared.projectId,
            projectRevision: prepared.revision,
            npcId: prepared.npcId,
            npcRevision: prepared.npcRevision,
            scriptModuleId: prepared.scriptModuleId,
            scriptModuleRevision: prepared.scriptModuleRevision,
            displayName: prepared.displayName,
            previousParentCatalogId: prepared.previousParentCatalogId,
            parentCatalogId: prepared.parentCatalogId,
            nameChanged: prepared.nameChanged,
            archetypeChanged: prepared.archetypeChanged,
            moduleRegenerated: prepared.moduleRegenerated,
          ),
        );
      },
    );
  }

  /// Create and publish one project-local DialogLine prerequisite without
  /// reading or writing a game installation or save. The exact native request
  /// is constructed only after entering the serialized managed-session lane.
  Future<ManagedRevision3DialogLineEntryCheckpoint>
  prepareAndPublishDialogLineV1({
    required Revision3DialogLineEntryTechnicalPlan plan,
  }) =>
      _core._publishPreparedRevision3Checkpoint<
        ManagedRevision3DialogLineEntryCheckpoint
      >(
        operation: 'prepareAndPublishDialogLineV1',
        handlePrepareError: _core._throwRevision3DialogLinePrepareError,
        prepare: (basis) async {
          final projectId = basis.projectId;
          final projectRevision = basis.projectRevision;
          if (projectId == null || projectRevision == null) {
            throw const ManagedProjectVerificationException(
              'revision-3 dialog-line transaction has no exact project identity',
            );
          }
          final request = AuthoringRevision3DialogLineEntryRequestV1.forProject(
            expectedHead: basis.head,
            currentProjectJson: basis.projectJson,
            lineId: plan.lineId,
            lineDisplayName: plan.lineDisplayName,
            lineAuthoredIdentity: plan.lineAuthoredIdentity,
            speakerHint: plan.speakerHint,
            localization: plan.localization,
            voiceSlot: plan.voiceSlot,
          );
          final prepared = await _store.prepareDialogLineV1(
            root: root.path,
            currentProjectJson: basis.projectJson,
            request: request,
          );
          if (prepared.basisHead.canonicalJson != basis.head.canonicalJson ||
              prepared.projectId != projectId ||
              prepared.revision != projectRevision + 1 ||
              prepared.lineId != plan.lineId ||
              prepared.localizationId != plan.localization.localizationId ||
              prepared.voiceSlotId != plan.voiceSlot?.slotId ||
              prepared.buildStatus !=
                  AuthoringRevision3DialogBuildStatus.blocked ||
              prepared.runtimeStatus !=
                  AuthoringRevision3DialogRuntimeStatus.runtimeUnqualified ||
              prepared.topicAuthority !=
                  AuthoringRevision3DialogTopicAuthority.notGranted ||
              prepared.publicationStatus !=
                  AuthoringRevision3DialogPublicationStatus.notSupported) {
            throw const ManagedProjectVerificationException(
              'revision-3 dialog-line preparation disagrees with its exact session basis or plan',
            );
          }
          return _ManagedPreparedCheckpoint<
            ManagedRevision3DialogLineEntryCheckpoint
          >(
            head: prepared.head,
            projectJson: prepared.projectJson,
            value: ManagedRevision3DialogLineEntryCheckpoint._(
              head: prepared.head,
              projectJson: prepared.projectJson,
              projectId: prepared.projectId,
              projectRevision: prepared.revision,
              lineId: prepared.lineId,
              localizationId: prepared.localizationId,
              localizationAction: prepared.localizationAction,
              voiceSlotId: prepared.voiceSlotId,
            ),
          );
        },
      );

  /// Read bounded per-locale previews from one exact managed LocalizationEntry.
  ///
  /// This read shares the serialized exact-head lane and carries neither game
  /// input nor project JSON across FFI. It prepares and publishes nothing.
  Future<AuthoringRevision3DialogLocalizationReadResult>
  readDialogLocalizationV1({
    required String localizationId,
    required int expectedLocalizationRevision,
    required String expectedLocId,
  }) => _core.readExact<AuthoringRevision3DialogLocalizationReadResult>(
    (basis) async {
      final projectId = basis.projectId;
      final projectRevision = basis.projectRevision;
      if (projectId == null || projectRevision == null) {
        throw const ManagedProjectVerificationException(
          'revision-3 dialog localization read has no exact project identity',
        );
      }
      final result = await _store.readDialogLocalizationV1(
        root: root.path,
        expectedHead: basis.head,
        localizationId: localizationId,
        expectedLocalizationRevision: expectedLocalizationRevision,
        expectedLocId: expectedLocId,
      );
      if (result.head.canonicalJson != basis.head.canonicalJson ||
          result.projectId != projectId ||
          result.projectRevision != projectRevision ||
          result.localizationId != localizationId ||
          result.localizationRevision != expectedLocalizationRevision ||
          result.locId != expectedLocId) {
        throw const ManagedProjectVerificationException(
          'revision-3 dialog localization read disagrees with its exact session basis',
        );
      }
      return result;
    },
    operation: 'readDialogLocalizationV1',
    handleReadError: _core._throwRevision3DialogLocalizationReadError,
  );

  /// Read one complete exact-current authored LocalizationEntry plus the
  /// bounded DialogLine/VoiceSlot facts needed to edit its locale texts.
  /// This operation prepares and publishes nothing.
  Future<AuthoringRevision3DialogLocalizationEditSeed>
  readDialogLocalizationEditSeedV1({
    required String localizationId,
    required int expectedLocalizationRevision,
    required String expectedLocId,
  }) => _core.readExact<AuthoringRevision3DialogLocalizationEditSeed>(
    (basis) async {
      final projectId = basis.projectId;
      final projectRevision = basis.projectRevision;
      if (projectId == null || projectRevision == null) {
        throw const ManagedProjectVerificationException(
          'revision-3 localization-edit seed has no exact project identity',
        );
      }
      final result = await _store.readDialogLocalizationEditSeedV1(
        root: root.path,
        expectedHead: basis.head,
        localizationId: localizationId,
        expectedLocalizationRevision: expectedLocalizationRevision,
        expectedLocId: expectedLocId,
      );
      if (result.head.canonicalJson != basis.head.canonicalJson ||
          result.projectId != projectId ||
          result.projectRevision != projectRevision ||
          result.localizationId != localizationId ||
          result.localizationRevision != expectedLocalizationRevision ||
          result.locId != expectedLocId ||
          result.contentAuthority !=
              AuthoringRevision3DialogLocalizationEditContentAuthority
                  .readOnlyExactCurrentLocalizationEditSeed ||
          result.buildStatus !=
              AuthoringRevision3DialogLocalizationEditSeedBuildStatus
                  .notEvaluated ||
          result.runtimeStatus !=
              AuthoringRevision3DialogLocalizationEditRuntimeStatus
                  .runtimeUnqualified ||
          result.publicationStatus !=
              AuthoringRevision3DialogLocalizationEditSeedPublicationStatus
                  .notApplicable) {
        throw const ManagedProjectVerificationException(
          'revision-3 localization-edit seed disagrees with its exact session basis',
        );
      }
      return result;
    },
    operation: 'readDialogLocalizationEditSeedV1',
    handleReadError: _core._throwRevision3DialogLocalizationEditReadError,
  );

  /// Replace the locale/text map of one exact authored LocalizationEntry.
  /// The typed request is rebound to the latest project JSON only inside the
  /// serialized managed lane before native prepares an unpublished candidate.
  Future<ManagedRevision3DialogLocalizationEditCheckpoint>
  prepareAndPublishDialogLocalizationEditV1({
    required Revision3DialogLocalizationEditTechnicalPlan plan,
  }) =>
      _core._publishPreparedRevision3Checkpoint<
        ManagedRevision3DialogLocalizationEditCheckpoint
      >(
        operation: 'prepareAndPublishDialogLocalizationEditV1',
        handlePrepareError:
            _core._throwRevision3DialogLocalizationEditPrepareError,
        prepare: (basis) async {
          final projectId = basis.projectId;
          final projectRevision = basis.projectRevision;
          if (projectId == null || projectRevision == null) {
            throw const ManagedProjectVerificationException(
              'revision-3 localization-edit transaction has no exact project identity',
            );
          }
          if (plan.expectedHead.canonicalJson != basis.head.canonicalJson) {
            throw const ManagedRevision3DialogLocalizationEditStaleException();
          }
          final request =
              AuthoringRevision3DialogLocalizationEditRequestV1.forProject(
                expectedHead: basis.head,
                currentProjectJson: basis.projectJson,
                localizationId: plan.localizationId,
                expectedLocalizationRevision: plan.expectedLocalizationRevision,
                expectedLocId: plan.expectedLocId,
                texts: plan.texts,
              );
          final prepared = await _store.prepareDialogLocalizationEditV1(
            root: root.path,
            currentProjectJson: basis.projectJson,
            request: request,
          );
          final currentTexts = _revision3DialogLocalizationTexts(
            basis.projectJson,
            localizationId: plan.localizationId,
            expectedLocalizationRevision: plan.expectedLocalizationRevision,
            expectedLocId: plan.expectedLocId,
          );
          final expectedAdded = plan.texts.keys
              .where((locale) => !currentTexts.containsKey(locale))
              .toList(growable: false);
          final expectedRemoved = currentTexts.keys
              .where((locale) => !plan.texts.containsKey(locale))
              .toList(growable: false);
          if (prepared.basisHead.canonicalJson != basis.head.canonicalJson ||
              prepared.projectId != projectId ||
              prepared.revision != projectRevision + 1 ||
              prepared.localizationId != plan.localizationId ||
              prepared.localizationRevision !=
                  plan.expectedLocalizationRevision + 1 ||
              !_sameStrings(prepared.addedLocales, expectedAdded) ||
              !_sameStrings(prepared.removedLocales, expectedRemoved) ||
              prepared.buildStatus !=
                  AuthoringRevision3DialogLocalizationEditBuildStatus.blocked ||
              prepared.runtimeStatus !=
                  AuthoringRevision3DialogLocalizationEditRuntimeStatus
                      .runtimeUnqualified ||
              prepared.topicAuthority !=
                  AuthoringRevision3DialogLocalizationEditTopicAuthority
                      .notGranted ||
              prepared.publicationStatus !=
                  AuthoringRevision3DialogLocalizationEditPublicationStatus
                      .notSupported) {
            throw const ManagedProjectVerificationException(
              'revision-3 localization-edit preparation disagrees with its exact session basis or plan',
            );
          }
          return _ManagedPreparedCheckpoint<
            ManagedRevision3DialogLocalizationEditCheckpoint
          >(
            head: prepared.head,
            projectJson: prepared.projectJson,
            value: ManagedRevision3DialogLocalizationEditCheckpoint._(
              head: prepared.head,
              projectJson: prepared.projectJson,
              projectId: prepared.projectId,
              projectRevision: prepared.revision,
              localizationId: prepared.localizationId,
              localizationRevision: prepared.localizationRevision,
              addedLocales: prepared.addedLocales,
              removedLocales: prepared.removedLocales,
            ),
          );
        },
      );

  /// Import and publish one revision-3 Ogg-backed VoiceTake for an existing line/locale.
  ///
  /// All request bindings are derived only after entering the serialized session lane. Native
  /// preparation may install immutable CAS objects but cannot replace the fixed head. The
  /// candidate is fully reopened before guarded publication and the published generation is
  /// fully reopened again before this method returns.
  Future<ManagedRevision3VoiceTakeCheckpoint> prepareAndPublishVoiceTakeV1({
    required String gameRoot,
    required String source,
    required String lineId,
    required String slotId,
    required String takeId,
    required String locale,
    String? text,
    required String takeDisplayName,
    required String logicalName,
    required AuthoringRevision3VoiceTakeStatus status,
    bool selectTake = false,
  }) => _core
      ._publishPreparedRevision3Checkpoint<ManagedRevision3VoiceTakeCheckpoint>(
        operation: 'prepareAndPublishVoiceTakeV1',
        handlePrepareError: _core._throwRevision3VoicePrepareError,
        prepare: (basis) async {
          final projectId = basis.projectId;
          final projectRevision = basis.projectRevision;
          if (projectId == null || projectRevision == null) {
            throw const ManagedProjectVerificationException(
              'revision-3 Voice transaction has no exact project identity',
            );
          }
          final request = AuthoringRevision3VoiceTakeRequestV1.forProject(
            expectedHead: basis.head,
            currentProjectJson: basis.projectJson,
            lineId: lineId,
            slotId: slotId,
            takeId: takeId,
            locale: locale,
            text: text,
            takeDisplayName: takeDisplayName,
            logicalName: logicalName,
            status: status,
            selectTake: selectTake,
          );
          final prepared = await _store.prepareVoiceTakeV1(
            root: root.path,
            gameRoot: gameRoot,
            source: source,
            currentProjectJson: basis.projectJson,
            request: request,
          );
          if (prepared.basisHead.canonicalJson != basis.head.canonicalJson ||
              prepared.projectId != projectId ||
              prepared.revision != projectRevision + 1 ||
              prepared.lineId != request.lineId ||
              prepared.slotId != request.slotId ||
              prepared.takeId != request.takeId ||
              prepared.locale != request.locale ||
              prepared.takeStatus != request.status ||
              prepared.selected != request.selectTake ||
              prepared.asset.logicalName != request.logicalName) {
            throw const ManagedProjectVerificationException(
              'revision-3 Voice preparation disagrees with its exact session basis or request',
            );
          }
          return _ManagedPreparedCheckpoint<
            ManagedRevision3VoiceTakeCheckpoint
          >(
            head: prepared.head,
            projectJson: prepared.projectJson,
            value: ManagedRevision3VoiceTakeCheckpoint._(
              head: prepared.head,
              projectJson: prepared.projectJson,
              projectId: prepared.projectId,
              projectRevision: prepared.revision,
              lineId: prepared.lineId,
              localizationId: prepared.localizationId,
              slotId: prepared.slotId,
              takeId: prepared.takeId,
              locale: prepared.locale,
              takeStatus: prepared.takeStatus,
              slotCreated: prepared.slotCreated,
              selected: prepared.selected,
              asset: prepared.asset,
              ogg: prepared.ogg,
              assetDeduplicated: prepared.assetDeduplicated,
            ),
          );
        },
      );

  /// Copy one exact-current managed CAS VoiceTake into a unique native-owned
  /// system-temp capability. The project head and revision remain unchanged.
  Future<Revision3VoiceTakePreviewCapability> materializeVoiceTakePreviewV1({
    required Revision3VoiceTakePreviewTechnicalPlan plan,
  }) async {
    final store = _store;
    if (store is! ManagedRevision3VoiceTakePreviewStore) {
      throw UnsupportedError(
        'this managed revision-3 Store has no Voice take preview capability',
      );
    }
    if (_core.requiresReopen) {
      throw const ManagedProjectVerificationException(
        'managed revision-3 Voice preview requires a verified reopen',
      );
    }
    final previewStore = store as ManagedRevision3VoiceTakePreviewStore;
    try {
      return await Revision3VoiceTakePreviewCapability.materialize(
        register: () =>
            previewStore.registerVoiceTakePreviewV1(root: root.path),
        release: (cleanupToken) =>
            previewStore.releaseVoiceTakePreviewV1(cleanupToken: cleanupToken),
        materialize: (cleanupToken, previewRoot) =>
            _core.readExact<AuthoringRevision3VoiceTakePreviewMaterialization>(
              (basis) async {
                final projectId = basis.projectId;
                final projectRevision = basis.projectRevision;
                if (projectId == null || projectRevision == null) {
                  throw UnsupportedError(
                    'this managed revision-3 Store has no Voice take preview capability',
                  );
                }
                final request = AuthoringRevision3VoiceTakePreviewRequestV1(
                  expectedHead: basis.head,
                  expectedProjectId: projectId,
                  expectedRevision: projectRevision,
                  lineId: plan.lineId,
                  expectedLineRevision: plan.expectedLineRevision,
                  localizationId: plan.localizationId,
                  expectedLocalizationRevision:
                      plan.expectedLocalizationRevision,
                  expectedLocId: plan.locId,
                  slotId: plan.slotId,
                  expectedSlotRevision: plan.expectedSlotRevision,
                  locale: plan.locale,
                  takeId: plan.takeId,
                  expectedTakeRevision: plan.expectedTakeRevision,
                  expectedAsset:
                      AuthoringRevision3VoiceTakePreviewExpectedAsset(
                        sha256: plan.assetSha256,
                        byteLength: plan.assetByteLength,
                        logicalName: plan.assetLogicalName,
                      ),
                );
                final result = await previewStore.materializeVoiceTakePreviewV1(
                  root: root.path,
                  cleanupToken: cleanupToken,
                  previewRoot: previewRoot,
                  request: request,
                );
                if (result.basisHead.canonicalJson !=
                        basis.head.canonicalJson ||
                    result.projectId != projectId ||
                    result.projectRevision != projectRevision ||
                    result.lineId != plan.lineId ||
                    result.lineRevision != plan.expectedLineRevision ||
                    result.localizationId != plan.localizationId ||
                    result.localizationRevision !=
                        plan.expectedLocalizationRevision ||
                    result.locId != plan.locId ||
                    result.slotId != plan.slotId ||
                    result.slotRevision != plan.expectedSlotRevision ||
                    result.locale != plan.locale ||
                    result.takeId != plan.takeId ||
                    result.takeRevision != plan.expectedTakeRevision ||
                    result.asset.sha256 != plan.assetSha256 ||
                    result.asset.byteLength != plan.assetByteLength ||
                    result.asset.logicalName != plan.assetLogicalName ||
                    result.status.name != plan.status.name ||
                    result.ogg.codec.name != plan.codec.name ||
                    result.ogg.channels != plan.channels ||
                    result.ogg.sampleRate != plan.sampleRate) {
                  throw const ManagedProjectVerificationException(
                    'revision-3 Voice preview disagrees with its exact session basis or plan',
                  );
                }
                return result;
              },
              operation: 'materializeVoiceTakePreviewV1',
              handleReadError: _core._throwRevision3VoicePreviewReadError,
            ),
      );
    } catch (error, stackTrace) {
      _core._throwRevision3VoicePreviewReadError(error, stackTrace);
    }
  }

  /// Inspect one exact-current managed VoiceTake without materializing bytes.
  /// The operation is serialized with project mutation and preserves the
  /// current checkpoint on success.
  Future<AuthoringRevision3VoiceTakeMediaQaResult> inspectVoiceTakeMediaQaV1({
    required Revision3VoiceTakePreviewTechnicalPlan plan,
  }) {
    final store = _store;
    if (store is! ManagedRevision3VoiceTakeMediaQaStore) {
      return Future<AuthoringRevision3VoiceTakeMediaQaResult>.error(
        UnsupportedError(
          'this managed revision-3 Store has no Voice take media QA capability',
        ),
      );
    }
    final mediaStore = store as ManagedRevision3VoiceTakeMediaQaStore;
    return _core.readExact<AuthoringRevision3VoiceTakeMediaQaResult>(
      (basis) async {
        final projectId = basis.projectId;
        final projectRevision = basis.projectRevision;
        if (projectId == null || projectRevision == null) {
          throw UnsupportedError(
            'this managed revision-3 Store has no Voice take media QA capability',
          );
        }
        final request = AuthoringRevision3VoiceTakePreviewRequestV1(
          expectedHead: basis.head,
          expectedProjectId: projectId,
          expectedRevision: projectRevision,
          lineId: plan.lineId,
          expectedLineRevision: plan.expectedLineRevision,
          localizationId: plan.localizationId,
          expectedLocalizationRevision: plan.expectedLocalizationRevision,
          expectedLocId: plan.locId,
          slotId: plan.slotId,
          expectedSlotRevision: plan.expectedSlotRevision,
          locale: plan.locale,
          takeId: plan.takeId,
          expectedTakeRevision: plan.expectedTakeRevision,
          expectedAsset: AuthoringRevision3VoiceTakePreviewExpectedAsset(
            sha256: plan.assetSha256,
            byteLength: plan.assetByteLength,
            logicalName: plan.assetLogicalName,
          ),
        );
        final result = await mediaStore.inspectVoiceTakeMediaV1(
          root: root.path,
          request: request,
        );
        final assuranceMatchesPlan = switch (plan.codec) {
          Revision3ContentVoiceOggCodec.vorbis =>
            result.assurance ==
                AuthoringRevision3VoiceTakeMediaAssurance.vorbisFullPcmDecode,
          Revision3ContentVoiceOggCodec.opus =>
            result.assurance ==
                AuthoringRevision3VoiceTakeMediaAssurance
                    .opusPacketAndTimingStructureOnly,
        };
        if (result.basisHead.canonicalJson != basis.head.canonicalJson ||
            result.projectId != projectId ||
            result.projectRevision != projectRevision ||
            result.lineId != plan.lineId ||
            result.lineRevision != plan.expectedLineRevision ||
            result.localizationId != plan.localizationId ||
            result.localizationRevision != plan.expectedLocalizationRevision ||
            result.locId != plan.locId ||
            result.slotId != plan.slotId ||
            result.slotRevision != plan.expectedSlotRevision ||
            result.locale != plan.locale ||
            result.takeId != plan.takeId ||
            result.takeRevision != plan.expectedTakeRevision ||
            result.asset.sha256 != plan.assetSha256 ||
            result.asset.byteLength != plan.assetByteLength ||
            result.asset.logicalName != plan.assetLogicalName ||
            result.status.name != plan.status.name ||
            result.ogg.codec.name != plan.codec.name ||
            result.ogg.channels != plan.channels ||
            result.ogg.sampleRate != plan.sampleRate ||
            result.duration.timebaseHz != plan.sampleRate ||
            !assuranceMatchesPlan ||
            result.mediaAuthority !=
                AuthoringRevision3VoiceTakeMediaAuthority
                    .exactCurrentManagedCasVoiceTakeMediaQaV1 ||
            result.inspectionScope !=
                AuthoringRevision3VoiceTakeMediaInspectionScope
                    .selectedVoiceTakeMediaInputOnly ||
            result.qualityStatus !=
                AuthoringRevision3VoiceTakeMediaEvaluationStatus.notEvaluated ||
            result.audibilityStatus !=
                AuthoringRevision3VoiceTakeMediaEvaluationStatus.notEvaluated ||
            result.projectWriteStatus !=
                AuthoringRevision3VoiceTakeMediaWriteStatus.notPerformed ||
            result.gameWriteStatus !=
                AuthoringRevision3VoiceTakeMediaWriteStatus.notPerformed ||
            result.saveWriteStatus !=
                AuthoringRevision3VoiceTakeMediaWriteStatus.notPerformed ||
            result.buildStatus !=
                AuthoringRevision3VoiceTakeMediaEvaluationStatus.notEvaluated ||
            result.deploymentStatus !=
                AuthoringRevision3VoiceTakeMediaDeploymentStatus.notPerformed ||
            result.runtimeStatus !=
                AuthoringRevision3VoiceTakeMediaRuntimeStatus.notQualified) {
          throw const ManagedProjectVerificationException(
            'revision-3 Voice media QA disagrees with its exact session basis or plan',
          );
        }
        return result;
      },
      operation: 'inspectVoiceTakeMediaQaV1',
      handleReadError: _core._throwRevision3VoiceMediaQaReadError,
    );
  }

  /// Read and seal one exact-current, filesystem-safe Voice folder plan.
  /// The operation is serialized with project mutation and rechecks the fixed
  /// head after the native scan. It writes no project, game, or save data.
  Future<AuthoringRevision3VoiceBatchPlanResult> planVoiceBatchV1({
    required String gameRoot,
    required String sourceFolder,
    required String locale,
  }) {
    if (_store is! ManagedRevision3VoiceBatchStore) {
      return Future<AuthoringRevision3VoiceBatchPlanResult>.error(
        UnsupportedError(
          'this managed revision-3 Store has no Voice batch capability',
        ),
      );
    }
    final batchStore = _store as ManagedRevision3VoiceBatchStore;
    return _core.readExact<AuthoringRevision3VoiceBatchPlanResult>(
      (basis) async {
        final projectId = basis.projectId;
        final projectRevision = basis.projectRevision;
        if (projectId == null || projectRevision == null) {
          throw const ManagedProjectVerificationException(
            'revision-3 Voice batch plan has no exact project identity',
          );
        }
        final result = await batchStore.planVoiceBatchV1(
          root: root.path,
          gameRoot: gameRoot,
          sourceFolder: sourceFolder,
          locale: locale,
          currentProjectJson: basis.projectJson,
          expectedHead: basis.head,
        );
        if (result.basisHead.canonicalJson != basis.head.canonicalJson ||
            result.projectId != projectId ||
            result.revision != projectRevision ||
            result.locale != locale) {
          throw const ManagedProjectVerificationException(
            'revision-3 Voice batch plan disagrees with its exact session basis',
          );
        }
        return result;
      },
      operation: 'planVoiceBatchV1',
      handleReadError: _core._throwRevision3VoiceBatchPlanError,
    );
  }

  /// Revalidate and publish every ready row from one exact folder plan as one
  /// project revision. Native preparation may retain verified immutable CAS
  /// orphans on failure, but the visible project graph is never partial.
  Future<ManagedRevision3VoiceBatchCheckpoint> prepareAndPublishVoiceBatchV1({
    required String gameRoot,
    required String sourceFolder,
    required AuthoringRevision3VoiceBatchPlanResult plan,
  }) {
    if (_store is! ManagedRevision3VoiceBatchStore) {
      return Future<ManagedRevision3VoiceBatchCheckpoint>.error(
        UnsupportedError(
          'this managed revision-3 Store has no Voice batch capability',
        ),
      );
    }
    final batchStore = _store as ManagedRevision3VoiceBatchStore;
    return _core._publishPreparedRevision3Checkpoint<
      ManagedRevision3VoiceBatchCheckpoint
    >(
      operation: 'prepareAndPublishVoiceBatchV1',
      handlePrepareError: _core._throwRevision3VoiceBatchPrepareError,
      prepare: (basis) async {
        final projectId = basis.projectId;
        final projectRevision = basis.projectRevision;
        if (projectId == null || projectRevision == null) {
          throw const ManagedProjectVerificationException(
            'revision-3 Voice batch transaction has no exact project identity',
          );
        }
        if (plan.basisHead.canonicalJson != basis.head.canonicalJson ||
            plan.projectId != projectId ||
            plan.revision != projectRevision ||
            !plan.canPrepare) {
          throw const Revision3VoiceBatchStaleCheckpointException();
        }
        final prepared = await batchStore.prepareVoiceBatchV1(
          root: root.path,
          gameRoot: gameRoot,
          sourceFolder: sourceFolder,
          currentProjectJson: basis.projectJson,
          plan: plan,
        );
        if (prepared.basisHead.canonicalJson != basis.head.canonicalJson ||
            prepared.projectId != projectId ||
            prepared.revision != projectRevision + 1 ||
            prepared.locale != plan.locale ||
            prepared.sourceManifestSha256 != plan.sourceManifestSha256 ||
            prepared.planSha256 != plan.planSha256 ||
            prepared.importedCount != plan.readyCount ||
            prepared.alreadyPresentCount != plan.alreadyPresentCount) {
          throw const ManagedProjectVerificationException(
            'revision-3 Voice batch preparation disagrees with its exact session plan',
          );
        }
        return _ManagedPreparedCheckpoint<ManagedRevision3VoiceBatchCheckpoint>(
          head: prepared.head,
          projectJson: prepared.projectJson,
          value: ManagedRevision3VoiceBatchCheckpoint._(
            head: prepared.head,
            projectJson: prepared.projectJson,
            projectId: prepared.projectId,
            projectRevision: prepared.revision,
            locale: prepared.locale,
            sourceManifestSha256: prepared.sourceManifestSha256,
            planSha256: prepared.planSha256,
            importedCount: prepared.importedCount,
            alreadyPresentCount: prepared.alreadyPresentCount,
            items: prepared.items,
          ),
        );
      },
    );
  }

  /// Select one existing Approved Voice take, or clear the current selection,
  /// through the project-only managed publication lane. No game root or media
  /// read is required or accepted.
  Future<ManagedRevision3VoiceTakeSelectionCheckpoint>
  prepareAndPublishVoiceTakeSelectionV1({
    required String lineId,
    required String slotId,
    required int expectedSlotRevision,
    required String locale,
    required String expectedLocId,
    required String? expectedSelectedTakeId,
    required String? selectedTakeId,
  }) =>
      _core._publishPreparedRevision3Checkpoint<
        ManagedRevision3VoiceTakeSelectionCheckpoint
      >(
        operation: 'prepareAndPublishVoiceTakeSelectionV1',
        handlePrepareError: _core._throwRevision3VoiceSelectionPrepareError,
        prepare: (basis) async {
          final projectId = basis.projectId;
          final projectRevision = basis.projectRevision;
          if (projectId == null || projectRevision == null) {
            throw const ManagedProjectVerificationException(
              'revision-3 Voice selection has no exact project identity',
            );
          }
          final request =
              AuthoringRevision3VoiceTakeSelectionRequestV1.forProject(
                expectedHead: basis.head,
                currentProjectJson: basis.projectJson,
                lineId: lineId,
                slotId: slotId,
                expectedSlotRevision: expectedSlotRevision,
                locale: locale,
                expectedLocId: expectedLocId,
                expectedSelectedTakeId: expectedSelectedTakeId,
                selectedTakeId: selectedTakeId,
              );
          final prepared = await _store.prepareVoiceTakeSelectionV1(
            root: root.path,
            currentProjectJson: basis.projectJson,
            request: request,
          );
          if (prepared.basisHead.canonicalJson != basis.head.canonicalJson ||
              prepared.projectId != projectId ||
              prepared.revision != projectRevision + 1 ||
              prepared.lineId != request.lineId ||
              prepared.slotId != request.slotId ||
              prepared.slotRevision != request.expectedSlotRevision + 1 ||
              prepared.locale != request.locale ||
              prepared.locId != request.expectedLocId ||
              prepared.previousSelectedTakeId !=
                  request.expectedSelectedTakeId ||
              prepared.selectedTakeId != request.selectedTakeId) {
            throw const ManagedProjectVerificationException(
              'revision-3 Voice selection preparation disagrees with its exact session basis or request',
            );
          }
          return _ManagedPreparedCheckpoint<
            ManagedRevision3VoiceTakeSelectionCheckpoint
          >(
            head: prepared.head,
            projectJson: prepared.projectJson,
            value: ManagedRevision3VoiceTakeSelectionCheckpoint._(
              head: prepared.head,
              projectJson: prepared.projectJson,
              projectId: prepared.projectId,
              projectRevision: prepared.revision,
              lineId: prepared.lineId,
              slotId: prepared.slotId,
              slotRevision: prepared.slotRevision,
              locale: prepared.locale,
              locId: prepared.locId,
              previousSelectedTakeId: prepared.previousSelectedTakeId,
              selectedTakeId: prepared.selectedTakeId,
            ),
          );
        },
      );

  /// Detach one exact VoiceTake candidate from one exact line/language slot
  /// through the project-only managed publication lane. Immutable audio CAS
  /// metadata is retained and no game, build, runtime, or deployment authority
  /// is accepted or produced.
  Future<ManagedRevision3VoiceTakeRemovalCheckpoint>
  prepareAndPublishVoiceTakeRemovalV1({
    required String lineId,
    required String localizationId,
    required String slotId,
    required int expectedSlotRevision,
    required String locale,
    required String expectedLocId,
    required String takeId,
    required int expectedTakeRevision,
    required String? expectedSelectedTakeId,
  }) {
    final removalStore = _store;
    if (removalStore is! ManagedRevision3VoiceTakeRemovalStore) {
      return Future<ManagedRevision3VoiceTakeRemovalCheckpoint>.error(
        UnsupportedError(
          'this managed revision-3 Store has no Voice take removal capability',
        ),
      );
    }
    final removalCapability =
        removalStore as ManagedRevision3VoiceTakeRemovalStore;
    return _core._publishPreparedRevision3Checkpoint<
      ManagedRevision3VoiceTakeRemovalCheckpoint
    >(
      operation: 'prepareAndPublishVoiceTakeRemovalV1',
      handlePrepareError: _core._throwRevision3VoiceTakeRemovalPrepareError,
      prepare: (basis) async {
        final projectId = basis.projectId;
        final projectRevision = basis.projectRevision;
        if (projectId == null || projectRevision == null) {
          throw const ManagedProjectVerificationException(
            'revision-3 Voice take removal has no exact project identity',
          );
        }
        // Keep the overflow rejection inside the serialized prepare lane but
        // before the Store call. The retryable closed error leaves this exact
        // session usable, matching ordinary compiler-like revision handling.
        if (projectRevision >= 0x7fffffffffffffff) {
          throw const ModFfiException(
            command: 'authoring_store_prepare_revision3_voice_take_removal_v1',
            code: 'AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_REVISION_LIMIT',
            message:
                'Voice take removal cannot advance the signed wire revision',
          );
        }
        final request = AuthoringRevision3VoiceTakeRemovalRequestV1.forProject(
          expectedHead: basis.head,
          currentProjectJson: basis.projectJson,
          lineId: lineId,
          localizationId: localizationId,
          expectedLocId: expectedLocId,
          locale: locale,
          slotId: slotId,
          expectedSlotRevision: expectedSlotRevision,
          takeId: takeId,
          expectedTakeRevision: expectedTakeRevision,
          expectedSelectedTakeId: expectedSelectedTakeId,
        );
        final prepared = await removalCapability.prepareVoiceTakeRemovalV1(
          root: root.path,
          currentProjectJson: basis.projectJson,
          request: request,
        );
        if (prepared.basisHead.canonicalJson != basis.head.canonicalJson ||
            prepared.projectId != projectId ||
            prepared.revision != projectRevision + 1 ||
            prepared.lineId != request.lineId ||
            prepared.localizationId != request.localizationId ||
            prepared.slotId != request.slotId ||
            prepared.slotRevision != request.expectedSlotRevision + 1 ||
            prepared.locale != request.locale ||
            prepared.locId != request.expectedLocId ||
            prepared.takeId != request.takeId ||
            prepared.takeRevision != request.expectedTakeRevision ||
            prepared.previousSelectedTakeId != request.expectedSelectedTakeId ||
            prepared.selectionCleared !=
                (request.expectedSelectedTakeId == request.takeId)) {
          throw const ManagedProjectVerificationException(
            'revision-3 Voice take removal preparation disagrees with its exact session basis or request',
          );
        }
        return _ManagedPreparedCheckpoint<
          ManagedRevision3VoiceTakeRemovalCheckpoint
        >(
          head: prepared.head,
          projectJson: prepared.projectJson,
          value: ManagedRevision3VoiceTakeRemovalCheckpoint._(
            head: prepared.head,
            projectJson: prepared.projectJson,
            projectId: prepared.projectId,
            projectRevision: prepared.revision,
            lineId: prepared.lineId,
            localizationId: prepared.localizationId,
            slotId: prepared.slotId,
            slotRevision: prepared.slotRevision,
            locale: prepared.locale,
            locId: prepared.locId,
            takeId: prepared.takeId,
            takeRevision: prepared.takeRevision,
            previousSelectedTakeId: prepared.previousSelectedTakeId,
            selectionCleared: prepared.selectionCleared,
            takeEntityRemoved: prepared.takeEntityRemoved,
            remainingCandidateCount: prepared.remainingCandidateCount,
          ),
        );
      },
    );
  }

  /// Remove one exact empty and unselected dialog VoiceSlot through the
  /// project-only managed publication lane.
  Future<ManagedRevision3DialogVoiceSlotRemovalCheckpoint>
  prepareAndPublishDialogVoiceSlotRemovalV1({
    required String lineId,
    required int expectedLineRevision,
    required String localizationId,
    required String slotId,
    required int expectedSlotRevision,
    required String locale,
    required String expectedLocId,
  }) {
    final removalStore = _store;
    if (removalStore is! ManagedRevision3DialogVoiceSlotRemovalStore) {
      return Future<ManagedRevision3DialogVoiceSlotRemovalCheckpoint>.error(
        UnsupportedError(
          'this managed revision-3 Store has no dialog Voice slot removal capability',
        ),
      );
    }
    final removalCapability =
        removalStore as ManagedRevision3DialogVoiceSlotRemovalStore;
    return _core._publishPreparedRevision3Checkpoint<
      ManagedRevision3DialogVoiceSlotRemovalCheckpoint
    >(
      operation: 'prepareAndPublishDialogVoiceSlotRemovalV1',
      handlePrepareError:
          _core._throwRevision3DialogVoiceSlotRemovalPrepareError,
      prepare: (basis) async {
        final projectId = basis.projectId;
        final projectRevision = basis.projectRevision;
        if (projectId == null || projectRevision == null) {
          throw const ManagedProjectVerificationException(
            'revision-3 dialog Voice slot removal has no exact project identity',
          );
        }
        if (projectRevision >= 0x7fffffffffffffff) {
          throw const ModFfiException(
            command:
                'authoring_store_prepare_revision3_dialog_voice_slot_removal_v1',
            code:
                'AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_REVISION_LIMIT',
            message:
                'dialog Voice slot removal cannot advance the signed wire revision',
          );
        }
        final request =
            AuthoringRevision3DialogVoiceSlotRemovalRequestV1.forProject(
              expectedHead: basis.head,
              currentProjectJson: basis.projectJson,
              lineId: lineId,
              expectedLineRevision: expectedLineRevision,
              localizationId: localizationId,
              expectedLocId: expectedLocId,
              locale: locale,
              slotId: slotId,
              expectedSlotRevision: expectedSlotRevision,
            );
        final prepared = await removalCapability
            .prepareDialogVoiceSlotRemovalV1(
              root: root.path,
              currentProjectJson: basis.projectJson,
              request: request,
            );
        if (prepared.basisHead.canonicalJson != basis.head.canonicalJson ||
            prepared.projectId != projectId ||
            prepared.revision != projectRevision + 1 ||
            prepared.lineId != request.lineId ||
            prepared.lineRevision != request.expectedLineRevision + 1 ||
            prepared.localizationId != request.localizationId ||
            prepared.slotId != request.slotId ||
            prepared.removedSlotRevision != request.expectedSlotRevision ||
            prepared.locale != request.locale ||
            prepared.locId != request.expectedLocId) {
          throw const ManagedProjectVerificationException(
            'revision-3 dialog Voice slot removal preparation disagrees with its exact session basis or request',
          );
        }
        return _ManagedPreparedCheckpoint<
          ManagedRevision3DialogVoiceSlotRemovalCheckpoint
        >(
          head: prepared.head,
          projectJson: prepared.projectJson,
          value: ManagedRevision3DialogVoiceSlotRemovalCheckpoint._(
            head: prepared.head,
            projectJson: prepared.projectJson,
            projectId: prepared.projectId,
            projectRevision: prepared.revision,
            lineId: prepared.lineId,
            lineRevision: prepared.lineRevision,
            localizationId: prepared.localizationId,
            slotId: prepared.slotId,
            removedSlotRevision: prepared.removedSlotRevision,
            locale: prepared.locale,
            locId: prepared.locId,
            removedTargetResolution: prepared.removedTargetResolution,
          ),
        );
      },
    );
  }

  /// Change one exact retained VoiceTake review status through the project-only
  /// managed publication lane. No game root, media, build, or runtime authority
  /// is required or accepted.
  Future<ManagedRevision3VoiceTakeStatusCheckpoint>
  prepareAndPublishVoiceTakeStatusV1({
    required String lineId,
    required String localizationId,
    required String expectedLocId,
    required String locale,
    required String slotId,
    required int expectedSlotRevision,
    required String takeId,
    required int expectedTakeRevision,
    required AuthoringRevision3VoiceTakeStatus expectedStatus,
    required AuthoringRevision3VoiceTakeStatus desiredStatus,
  }) =>
      _core._publishPreparedRevision3Checkpoint<
        ManagedRevision3VoiceTakeStatusCheckpoint
      >(
        operation: 'prepareAndPublishVoiceTakeStatusV1',
        handlePrepareError: _core._throwRevision3VoiceTakeStatusPrepareError,
        prepare: (basis) async {
          final projectId = basis.projectId;
          final projectRevision = basis.projectRevision;
          if (projectId == null || projectRevision == null) {
            throw const ManagedProjectVerificationException(
              'revision-3 Voice take status has no exact project identity',
            );
          }
          final request = AuthoringRevision3VoiceTakeStatusRequestV1.forProject(
            expectedHead: basis.head,
            currentProjectJson: basis.projectJson,
            lineId: lineId,
            localizationId: localizationId,
            expectedLocId: expectedLocId,
            locale: locale,
            slotId: slotId,
            expectedSlotRevision: expectedSlotRevision,
            takeId: takeId,
            expectedTakeRevision: expectedTakeRevision,
            expectedStatus: expectedStatus,
            desiredStatus: desiredStatus,
          );
          final prepared = await _store.prepareVoiceTakeStatusV1(
            root: root.path,
            currentProjectJson: basis.projectJson,
            request: request,
          );
          if (prepared.basisHead.canonicalJson != basis.head.canonicalJson ||
              prepared.projectId != projectId ||
              prepared.revision != projectRevision + 1 ||
              prepared.lineId != request.lineId ||
              prepared.localizationId != request.localizationId ||
              prepared.slotId != request.slotId ||
              prepared.slotRevision != request.expectedSlotRevision ||
              prepared.locale != request.locale ||
              prepared.locId != request.expectedLocId ||
              prepared.takeId != request.takeId ||
              prepared.takeRevision != request.expectedTakeRevision + 1 ||
              prepared.previousStatus != request.expectedStatus ||
              prepared.status != request.desiredStatus ||
              prepared.buildStatus !=
                  AuthoringRevision3VoiceTakeStatusBuildStatus.blocked ||
              prepared.runtimeStatus !=
                  AuthoringRevision3VoiceTakeStatusRuntimeStatus
                      .runtimeUnqualified ||
              prepared.publicationStatus !=
                  AuthoringRevision3VoiceTakeStatusPublicationStatus
                      .notSupported) {
            throw const ManagedProjectVerificationException(
              'revision-3 Voice take status preparation disagrees with its exact session basis or request',
            );
          }
          return _ManagedPreparedCheckpoint<
            ManagedRevision3VoiceTakeStatusCheckpoint
          >(
            head: prepared.head,
            projectJson: prepared.projectJson,
            value: ManagedRevision3VoiceTakeStatusCheckpoint._(
              head: prepared.head,
              projectJson: prepared.projectJson,
              projectId: prepared.projectId,
              projectRevision: prepared.revision,
              lineId: prepared.lineId,
              localizationId: prepared.localizationId,
              slotId: prepared.slotId,
              slotRevision: prepared.slotRevision,
              locale: prepared.locale,
              locId: prepared.locId,
              takeId: prepared.takeId,
              takeRevision: prepared.takeRevision,
              previousStatus: prepared.previousStatus,
              status: prepared.status,
            ),
          );
        },
      );

  /// Resolve one exact VoiceSlot against the installed locale archive and
  /// publish only the resulting sealed unresolved/resolved/ambiguous evidence.
  Future<ManagedRevision3VoiceTargetCheckpoint> prepareAndPublishVoiceTargetV1({
    required String gameRoot,
    required String lineId,
    required String slotId,
    required String locale,
    required String expectedLocId,
  }) =>
      _core._publishPreparedRevision3Checkpoint<
        ManagedRevision3VoiceTargetCheckpoint
      >(
        operation: 'prepareAndPublishVoiceTargetV1',
        handlePrepareError: _core._throwRevision3VoiceTargetPrepareError,
        prepare: (basis) async {
          final projectId = basis.projectId;
          final projectRevision = basis.projectRevision;
          if (projectId == null || projectRevision == null) {
            throw const ManagedProjectVerificationException(
              'revision-3 Voice target transaction has no exact project identity',
            );
          }
          final request = AuthoringRevision3VoiceTargetRequestV1.forProject(
            expectedHead: basis.head,
            currentProjectJson: basis.projectJson,
            lineId: lineId,
            slotId: slotId,
            locale: locale,
            expectedLocId: expectedLocId,
          );
          final prepared = await _store.prepareVoiceTargetV1(
            root: root.path,
            gameRoot: gameRoot,
            currentProjectJson: basis.projectJson,
            request: request,
          );
          if (prepared.basisHead.canonicalJson != basis.head.canonicalJson ||
              prepared.projectId != projectId ||
              prepared.revision != projectRevision + 1 ||
              prepared.lineId != request.lineId ||
              prepared.slotId != request.slotId ||
              prepared.locale != request.locale ||
              prepared.locId != request.expectedLocId) {
            throw const ManagedProjectVerificationException(
              'revision-3 Voice target preparation disagrees with its exact session basis or request',
            );
          }
          return _ManagedPreparedCheckpoint<
            ManagedRevision3VoiceTargetCheckpoint
          >(
            head: prepared.head,
            projectJson: prepared.projectJson,
            value: ManagedRevision3VoiceTargetCheckpoint._(
              head: prepared.head,
              projectJson: prepared.projectJson,
              projectId: prepared.projectId,
              projectRevision: prepared.revision,
              lineId: prepared.lineId,
              localizationId: prepared.localizationId,
              slotId: prepared.slotId,
              locale: prepared.locale,
              locId: prepared.locId,
              resolution: prepared.resolution,
              targets: prepared.targets,
              archiveObservation: prepared.archiveObservation,
            ),
          );
        },
      );

  /// Evaluate Voice build readiness against the exact current checkpoint.
  ///
  /// This is a serialized exact-head read. It accepts no game or output path,
  /// writes nothing, and grants neither build nor deployment authority.
  Future<AuthoringRevision3VoiceBuildPlanResult>
  planVoiceV1() => _core.readExact<AuthoringRevision3VoiceBuildPlanResult>(
    (basis) async {
      final projectId = basis.projectId;
      final projectRevision = basis.projectRevision;
      if (projectId == null || projectRevision == null) {
        throw const ManagedProjectVerificationException(
          'revision-3 Voice build plan has no exact project identity',
        );
      }
      final result = await _store.planVoiceV1(
        root: root.path,
        currentProjectJson: basis.projectJson,
        expectedHead: basis.head,
      );
      if (result.basisHead.canonicalJson != basis.head.canonicalJson ||
          result.projectId != projectId ||
          result.projectRevision != projectRevision) {
        throw const ManagedProjectVerificationException(
          'revision-3 Voice build plan disagrees with its exact session basis',
        );
      }
      return result;
    },
    operation: 'planVoiceV1',
    handleReadError: _core._throwRevision3VoiceBuildPlanError,
  );

  /// Build the exact current selected Voice graph into a new offline bundle.
  /// This is a serialized exact-head read and never publishes or deploys.
  Future<AuthoringRevision3VoiceBuildResult> buildVoiceV1({
    required String gameRoot,
    required String output,
  }) => _core.readBasisSnapshot<AuthoringRevision3VoiceBuildResult>(
    (basis) async {
      final projectId = basis.projectId;
      final projectRevision = basis.projectRevision;
      if (projectId == null || projectRevision == null) {
        throw const ManagedProjectVerificationException(
          'revision-3 Voice build has no exact project identity',
        );
      }
      final result = await _store.buildVoiceV1(
        root: root.path,
        gameRoot: gameRoot,
        currentProjectJson: basis.projectJson,
        expectedHead: basis.head,
        output: output,
      );
      if (result.basisHead.canonicalJson != basis.head.canonicalJson ||
          result.projectId != projectId ||
          result.projectRevision != projectRevision) {
        throw const ManagedProjectVerificationException(
          'revision-3 Voice build disagrees with its exact session basis',
        );
      }
      return result;
    },
    operation: 'buildVoiceV1',
    handleReadError: _core._throwRevision3VoiceBuildError,
  );

  /// Export the exact captured revision-3 basis as an immutable portable
  /// review snapshot. This operation is serialized with edits but does not
  /// publish a project checkpoint or mutate session state.
  Future<AuthoringRevision3ExactSnapshotExportResult> exportExactSnapshotV1({
    required String output,
  }) {
    final exportStore = _store;
    if (exportStore is! ManagedRevision3ExactSnapshotExportStore) {
      return Future<AuthoringRevision3ExactSnapshotExportResult>.error(
        UnsupportedError(
          'this managed revision-3 Store cannot export exact snapshots',
        ),
      );
    }
    final exactExportStore =
        exportStore as ManagedRevision3ExactSnapshotExportStore;
    return _core.readBasisSnapshot<AuthoringRevision3ExactSnapshotExportResult>(
      (basis) async {
        final projectId = basis.projectId;
        final projectRevision = basis.projectRevision;
        if (projectId == null || projectRevision == null) {
          throw const ManagedProjectVerificationException(
            'revision-3 exact snapshot export has no exact project identity',
          );
        }
        final result = await exactExportStore.exportExactSnapshotV1(
          root: root.path,
          expectedHead: basis.head,
          output: output,
        );
        if (result.basisHead.canonicalJson != basis.head.canonicalJson ||
            result.projectId != projectId ||
            result.projectRevision != projectRevision ||
            result.output != output) {
          throw const ManagedProjectVerificationException(
            'revision-3 exact snapshot export disagrees with its exact session basis or output',
          );
        }
        return result;
      },
      operation: 'exportExactSnapshotV1',
      handleReadError: _core._throwRevision3ExactSnapshotExportError,
    );
  }

  /// Build one exact-basis, reviewed DataAsset stage into a new immutable
  /// offline triplet. Publication uncertainty is retained as a successful,
  /// sealed result so callers do not accidentally retry a completed rename.
  Future<AuthoringRevision3ReviewedDataAssetBuildResult>
  buildReviewedDataAssetV1({
    required String gameRoot,
    required String targetPath,
    required String packName,
    required String output,
  }) {
    if (!supportsReviewedDataAssetBuild) {
      return Future<AuthoringRevision3ReviewedDataAssetBuildResult>.error(
        UnsupportedError(
          'this managed revision-3 Store cannot build reviewed DataAssets',
        ),
      );
    }
    return _core.readBasisSnapshot<
      AuthoringRevision3ReviewedDataAssetBuildResult
    >(
      (basis) async {
        final projectId = basis.projectId;
        final projectRevision = basis.projectRevision;
        if (projectId == null || projectRevision == null) {
          throw const ManagedProjectVerificationException(
            'revision-3 reviewed DataAsset build has no exact project identity',
          );
        }
        final result = await _core._store.buildReviewedDataAssetV1(
          root: root.path,
          gameRoot: gameRoot,
          currentProjectJson: basis.projectJson,
          expectedHead: basis.head,
          targetPath: targetPath,
          packName: packName,
          output: output,
        );
        if (result.basisHead.canonicalJson != basis.head.canonicalJson ||
            result.projectId != projectId ||
            result.projectRevision != projectRevision ||
            result.targetPath != targetPath ||
            result.packName != packName ||
            result.output != output) {
          throw const ManagedProjectVerificationException(
            'reviewed DataAsset build disagrees with its exact session basis or intent',
          );
        }
        return result;
      },
      operation: 'buildReviewedDataAssetV1',
      handleReadError: _core._throwRevision3ReviewedDataAssetBuildError,
    );
  }

  /// Verify a PatchReceipt-v2 input and publish its closed fixed-leaf DataAsset stage through the
  /// session's existing full-reopen, crash-repair and exact byte-CAS lane.
  Future<ManagedRevision3DataAssetStageCheckpoint>
  prepareAndPublishDataAssetStageV1({required String patchReceiptPath}) =>
      _prepareAndPublishDataAssetStage(
        operation: 'prepareAndPublishDataAssetStageV1',
        prepare: (basis) => _store.prepareDataAssetStageV1(
          root: root.path,
          expectedHead: basis.head,
          patchReceiptPath: patchReceiptPath,
        ),
      );

  /// Encode and verify one typed fixed-leaf value against its exact
  /// ExtractReceipt-v2, then publish the closed stage through the same guarded
  /// full-reopen and fixed-head byte-CAS lane as receipt imports.
  Future<ManagedRevision3DataAssetStageCheckpoint>
  prepareAndPublishDataAssetEditV1({
    required DataAssetSemanticEditIntent intent,
  }) => _prepareAndPublishDataAssetStage(
    operation: 'prepareAndPublishDataAssetEditV1',
    prepare: (basis) => _store.prepareDataAssetEditV1(
      root: root.path,
      expectedHead: basis.head,
      intent: intent,
    ),
  );

  /// Revalidate one exact installed snapshot and its inspection, independently
  /// reconstruct the selected package, then publish only the resulting closed
  /// stage through the existing fixed-head byte-CAS lane.
  Future<ManagedRevision3DataAssetStageCheckpoint>
  prepareAndPublishInstalledDataAssetEditV1({
    required String gameRoot,
    required DataAssetInstalledSemanticEditIntent intent,
  }) => _prepareAndPublishDataAssetStage(
    operation: 'prepareAndPublishInstalledDataAssetEditV1',
    handlePrepareError: _core._throwRevision3InstalledDataAssetEditError,
    prepare: (basis) {
      if (intent.snapshot.head.canonicalJson != basis.head.canonicalJson ||
          intent.snapshot.projectId != basis.projectId ||
          intent.snapshot.projectRevision != basis.projectRevision ||
          intent.inspection.head.canonicalJson != basis.head.canonicalJson ||
          intent.inspection.projectId != basis.projectId ||
          intent.inspection.projectRevision != basis.projectRevision) {
        throw const ManagedProjectVerificationException(
          'installed DataAsset edit is not bound to the exact session basis',
        );
      }
      return _store.prepareInstalledDataAssetEditV1(
        root: root.path,
        gameRoot: gameRoot,
        expectedHead: basis.head,
        intent: intent,
      );
    },
  );

  /// Revalidate and natively lower one closed reviewed installed DataAsset
  /// intent, then publish only its prepared stage through fixed-head byte CAS.
  Future<ManagedRevision3DataAssetStageCheckpoint>
  prepareAndPublishReviewedInstalledDataAssetEditV1({
    required String gameRoot,
    required ReviewedInstalledDataAssetEditIntent intent,
  }) => _prepareAndPublishDataAssetStage(
    operation: 'prepareAndPublishReviewedInstalledDataAssetEditV1',
    handlePrepareError: _core._throwRevision3InstalledDataAssetEditError,
    prepare: (basis) {
      if (intent.snapshot.head.canonicalJson != basis.head.canonicalJson ||
          intent.snapshot.projectId != basis.projectId ||
          intent.snapshot.projectRevision != basis.projectRevision ||
          intent.inspection.head.canonicalJson != basis.head.canonicalJson ||
          intent.inspection.projectId != basis.projectId ||
          intent.inspection.projectRevision != basis.projectRevision) {
        throw const ManagedProjectVerificationException(
          'reviewed installed DataAsset edit is not bound to the exact session basis',
        );
      }
      return _store.prepareReviewedInstalledDataAssetEditV1(
        root: root.path,
        gameRoot: gameRoot,
        expectedHead: basis.head,
        intent: intent,
      );
    },
  );

  Future<ManagedRevision3DataAssetStageCheckpoint>
  _prepareAndPublishDataAssetStage({
    required String operation,
    required Future<AuthoringRevision3DataAssetStagePreparation> Function(
      _ManagedOpenedCheckpoint basis,
    )
    prepare,
    Never Function(Object error, StackTrace stackTrace)? handlePrepareError,
  }) =>
      _core._publishPreparedRevision3Checkpoint<
        ManagedRevision3DataAssetStageCheckpoint
      >(
        operation: operation,
        handlePrepareError:
            handlePrepareError ?? _core._throwRevision3DataAssetError,
        prepare: (basis) async {
          final projectId = basis.projectId;
          final projectRevision = basis.projectRevision;
          if (projectId == null || projectRevision == null) {
            throw const ManagedProjectVerificationException(
              'revision-3 DataAsset transaction has no exact project identity',
            );
          }
          final prepared = await prepare(basis);
          final stage = prepared.stage;
          if (prepared.basisHead.canonicalJson != basis.head.canonicalJson ||
              prepared.projectId != projectId ||
              prepared.revision != projectRevision + 1 ||
              stage.projectId != projectId ||
              stage.basisHead.canonicalJson != basis.head.canonicalJson ||
              stage.basisProjectRevision != projectRevision ||
              stage.stagedProjectRevision != prepared.revision) {
            throw const ManagedProjectVerificationException(
              'revision-3 DataAsset preparation disagrees with its exact session basis',
            );
          }
          return _ManagedPreparedCheckpoint<
            ManagedRevision3DataAssetStageCheckpoint
          >(
            head: prepared.head,
            projectJson: prepared.projectJson,
            value: ManagedRevision3DataAssetStageCheckpoint._(
              head: prepared.head,
              projectJson: prepared.projectJson,
              projectId: prepared.projectId,
              projectRevision: prepared.revision,
              stage: stage,
              deduplicatedBlobs: prepared.deduplicatedBlobs,
            ),
          );
        },
      );

  /// Read the exact current managed DataAsset stage registry without preparing or publishing.
  Future<List<AuthoringRevision3DataAssetStage>> listDataAssetStagesV1() =>
      _core.readExact<List<AuthoringRevision3DataAssetStage>>(
        (basis) async {
          final projectId = basis.projectId;
          final projectRevision = basis.projectRevision;
          if (projectId == null || projectRevision == null) {
            throw const ManagedProjectVerificationException(
              'revision-3 DataAsset read has no exact project identity',
            );
          }
          final result = await _store.listDataAssetStagesV1(
            root: root.path,
            expectedHead: basis.head,
          );
          if (result.basisHead.canonicalJson != basis.head.canonicalJson ||
              result.revision != projectRevision ||
              result.stages.any(
                (stage) =>
                    stage.projectId != projectId ||
                    stage.stagedProjectRevision > projectRevision,
              )) {
            throw const ManagedProjectVerificationException(
              'revision-3 DataAsset list disagrees with its exact session basis',
            );
          }
          return result.stages;
        },
        operation: 'listDataAssetStagesV1',
        handleReadError: _core._throwRevision3DataAssetError,
      );

  /// Remove one managed stage registry entry through the guarded fixed-head publication lane.
  Future<ManagedRevision3DataAssetStageRemovalCheckpoint>
  prepareAndPublishRemoveDataAssetStageV1({required String targetPath}) =>
      _core._publishPreparedRevision3Checkpoint<
        ManagedRevision3DataAssetStageRemovalCheckpoint
      >(
        operation: 'prepareAndPublishRemoveDataAssetStageV1',
        handlePrepareError: _core._throwRevision3DataAssetError,
        prepare: (basis) async {
          final projectId = basis.projectId;
          final projectRevision = basis.projectRevision;
          if (projectId == null || projectRevision == null) {
            throw const ManagedProjectVerificationException(
              'revision-3 DataAsset removal has no exact project identity',
            );
          }
          final prepared = await _store.prepareRemoveDataAssetStageV1(
            root: root.path,
            expectedHead: basis.head,
            targetPath: targetPath,
          );
          if (prepared.basisHead.canonicalJson != basis.head.canonicalJson ||
              prepared.projectId != projectId ||
              prepared.revision != projectRevision + 1 ||
              prepared.removed.projectId != projectId ||
              prepared.removed.targetPath.toLowerCase() !=
                  targetPath.toLowerCase() ||
              prepared.removed.stagedProjectRevision > projectRevision) {
            throw const ManagedProjectVerificationException(
              'revision-3 DataAsset removal disagrees with its exact session basis',
            );
          }
          return _ManagedPreparedCheckpoint<
            ManagedRevision3DataAssetStageRemovalCheckpoint
          >(
            head: prepared.head,
            projectJson: prepared.projectJson,
            value: ManagedRevision3DataAssetStageRemovalCheckpoint._(
              head: prepared.head,
              projectJson: prepared.projectJson,
              projectId: prepared.projectId,
              projectRevision: prepared.revision,
              removed: prepared.removed,
            ),
          );
        },
      );

  /// Remove one exact NPC/Quest Draft together with only its uniquely-owned
  /// generated ScriptModule. Preparation remains unpublished until the
  /// candidate passes the ordinary full-reopen and fixed-head CAS lane.
  Future<ManagedRevision3StoryDraftRemovalCheckpoint>
  prepareAndPublishRemoveStoryDraftV1({
    required String draftId,
    required AuthoringStoryDraftKind draftKind,
    required int expectedDraftRevision,
    required String scriptModuleId,
    required int expectedScriptModuleRevision,
  }) {
    final removalStore = _store;
    if (removalStore is! ManagedRevision3StoryDraftRemovalStore) {
      return Future<ManagedRevision3StoryDraftRemovalCheckpoint>.error(
        UnsupportedError(
          'this managed revision-3 Store has no Story Draft removal capability',
        ),
      );
    }
    final removalCapability =
        removalStore as ManagedRevision3StoryDraftRemovalStore;
    if (!_managedRevision3CompilerIdPattern.hasMatch(draftId) ||
        draftId == _managedRevision3CompilerZeroId ||
        !_managedRevision3CompilerIdPattern.hasMatch(scriptModuleId) ||
        scriptModuleId == _managedRevision3CompilerZeroId ||
        draftId == scriptModuleId ||
        expectedDraftRevision < 0 ||
        expectedDraftRevision > 0x7fffffffffffffff ||
        expectedScriptModuleRevision < 0 ||
        expectedScriptModuleRevision > 0x7fffffffffffffff) {
      return Future<ManagedRevision3StoryDraftRemovalCheckpoint>.error(
        ArgumentError(
          'Story Draft removal requires distinct nonzero IDs and signed-safe revisions',
        ),
      );
    }
    return _core._publishPreparedRevision3Checkpoint<
      ManagedRevision3StoryDraftRemovalCheckpoint
    >(
      operation: 'prepareAndPublishRemoveStoryDraftV1',
      handlePrepareError: _core._throwRevision3StoryDraftRemovalError,
      prepare: (basis) async {
        final projectId = basis.projectId;
        final projectRevision = basis.projectRevision;
        if (projectId == null || projectRevision == null) {
          throw const ManagedProjectVerificationException(
            'revision-3 Story Draft removal has no exact project identity',
          );
        }
        if (projectRevision >= 0x7fffffffffffffff) {
          throw const ModFfiException(
            command: 'authoring_store_prepare_remove_revision3_story_draft_v1',
            code: 'AUTHORING_REVISION3_STORY_DRAFT_REMOVE_REVISION_LIMIT',
            message:
                'Story Draft removal cannot advance the signed wire revision',
          );
        }
        final request = AuthoringRevision3StoryDraftRemovalRequestV1.forProject(
          currentProjectJson: basis.projectJson,
          expectedHead: basis.head,
          draftId: draftId,
          draftKind: draftKind,
          expectedDraftRevision: expectedDraftRevision,
          scriptModuleId: scriptModuleId,
          expectedScriptModuleRevision: expectedScriptModuleRevision,
        );
        final prepared = await removalCapability.prepareRemoveStoryDraftV1(
          root: root.path,
          currentProjectJson: basis.projectJson,
          request: request,
        );
        if (prepared.basisHead.canonicalJson != basis.head.canonicalJson ||
            prepared.projectId != projectId ||
            prepared.revision != projectRevision + 1 ||
            prepared.removedDraft.id != draftId ||
            prepared.removedDraft.kind != draftKind ||
            prepared.removedDraft.revision != expectedDraftRevision ||
            prepared.removedScriptModule.id != scriptModuleId ||
            prepared.removedScriptModule.revision !=
                expectedScriptModuleRevision) {
          throw const ManagedProjectVerificationException(
            'revision-3 Story Draft removal disagrees with its exact session basis or request',
          );
        }
        return _ManagedPreparedCheckpoint<
          ManagedRevision3StoryDraftRemovalCheckpoint
        >(
          head: prepared.head,
          projectJson: prepared.projectJson,
          value: ManagedRevision3StoryDraftRemovalCheckpoint._(
            head: prepared.head,
            projectJson: prepared.projectJson,
            projectId: prepared.projectId,
            projectRevision: prepared.revision,
            removedDraftId: prepared.removedDraft.id,
            removedDraftKind: prepared.removedDraft.kind,
            removedDraftRevision: prepared.removedDraft.revision,
            removedScriptModuleId: prepared.removedScriptModule.id,
            removedScriptModuleRevision: prepared.removedScriptModule.revision,
          ),
        );
      },
    );
  }

  /// Derive one private effective transition-plan seed from this session's
  /// exact canonical project. Legacy plans are synthesized deterministically;
  /// no project bytes leave the serialized read lane and no candidate is
  /// prepared or published.
  Future<AuthoringRevision3QuestTransitionsSeed> readQuestTransitionsSeedV1({
    required String questId,
    required int expectedQuestRevision,
    required String expectedModuleId,
    required int expectedModuleRevision,
  }) => _core.readExact<AuthoringRevision3QuestTransitionsSeed>(
    (basis) async => AuthoringRevision3QuestTransitionsSeed.forProject(
      currentProjectJson: basis.projectJson,
      questId: questId,
      expectedQuestRevision: expectedQuestRevision,
      expectedModuleId: expectedModuleId,
      expectedModuleRevision: expectedModuleRevision,
    ),
    operation: 'readQuestTransitionsSeedV1',
    handleReadError: _core._throwRevision3QuestTransitionsSeedReadError,
  );

  /// Read one private existing-Quest context seed directly from this session's
  /// exact canonical project. The project transport never leaves the session.
  Future<AuthoringRevision3QuestContextSeed> readQuestContextSeedV1({
    required String questId,
    required int expectedQuestRevision,
    required String expectedModuleId,
    required int expectedModuleRevision,
    required String expectedParentRuntimeClass,
    required String expectedGiverRuntimeUniqueName,
  }) => _core.readExact<AuthoringRevision3QuestContextSeed>(
    (basis) async => AuthoringRevision3QuestContextSeed.forProject(
      currentProjectJson: basis.projectJson,
      questId: questId,
      expectedQuestRevision: expectedQuestRevision,
      expectedModuleId: expectedModuleId,
      expectedModuleRevision: expectedModuleRevision,
      expectedParentRuntimeClass: expectedParentRuntimeClass,
      expectedGiverRuntimeUniqueName: expectedGiverRuntimeUniqueName,
    ),
    operation: 'readQuestContextSeedV1',
    handleReadError: _core._throwRevision3QuestContextSeedReadError,
  );

  /// Reconstruct and verify the generated source for one exact-current Quest.
  ///
  /// Native code receives only the Store root, read-only game root, exact head
  /// and selected Quest ID. This serialized read lane checks the published head
  /// on both sides and never prepares or publishes a checkpoint.
  Future<AuthoringRevision3QuestSourceInspectionResult> inspectQuestSourceV1({
    required String gameRoot,
    required String questId,
  }) => _core.readExact<AuthoringRevision3QuestSourceInspectionResult>(
    (basis) async {
      final projectId = basis.projectId;
      final projectRevision = basis.projectRevision;
      if (projectId == null || projectRevision == null) {
        throw const ManagedProjectVerificationException(
          'revision-3 Quest source inspection has no exact project identity',
        );
      }
      final result = await _store.inspectQuestSourceV1(
        root: root.path,
        gameRoot: gameRoot,
        expectedHead: basis.head,
        questId: questId,
      );
      final projectBytes = utf8.encode(basis.projectJson);
      if (result.head.canonicalJson != basis.head.canonicalJson ||
          result.projectId != projectId ||
          result.projectRevision != projectRevision ||
          result.questId != questId ||
          result.projectSeal.byteLength != projectBytes.length ||
          result.projectSeal.sha256 !=
              crypto.sha256.convert(projectBytes).toString()) {
        throw const ManagedProjectVerificationException(
          'revision-3 Quest source inspection disagrees with its exact session basis',
        );
      }
      return result;
    },
    operation: 'inspectQuestSourceV1',
    handleReadError: _core._throwRevision3QuestSourceInspectionError,
  );

  /// Verify persisted source and readiness evidence for one exact-current NPC
  /// Draft. This project-only read never prepares or publishes a checkpoint and
  /// does not require a configured game installation.
  Future<AuthoringRevision3NpcSourceInspectionResult> inspectNpcSourceV1({
    required String npcId,
  }) => _core.readExact<AuthoringRevision3NpcSourceInspectionResult>(
    (basis) async {
      final projectId = basis.projectId;
      final projectRevision = basis.projectRevision;
      if (projectId == null || projectRevision == null) {
        throw const ManagedProjectVerificationException(
          'revision-3 NPC source inspection has no exact project identity',
        );
      }
      final result = await _store.inspectNpcSourceV1(
        root: root.path,
        expectedHead: basis.head,
        npcId: npcId,
      );
      final projectBytes = utf8.encode(basis.projectJson);
      if (result.head.canonicalJson != basis.head.canonicalJson ||
          result.projectId != projectId ||
          result.projectRevision != projectRevision ||
          result.npcId != npcId ||
          result.projectSeal.byteLength != projectBytes.length ||
          result.projectSeal.sha256 !=
              crypto.sha256.convert(projectBytes).toString()) {
        throw const ManagedProjectVerificationException(
          'revision-3 NPC source inspection disagrees with its exact session basis',
        );
      }
      return result;
    },
    operation: 'inspectNpcSourceV1',
    handleReadError: _core._throwRevision3NpcSourceInspectionError,
  );

  /// Run the game compiler against one native-derived Quest/NPC module from
  /// the exact serialized Store basis. Only the selected entity ID crosses the
  /// wire; the expected revisions and module identity are caller-side stale
  /// selection guards and are re-derived from [projectJson] in this lane.
  ///
  /// Native discards the compiled mini-cache before returning. A post-call
  /// fixed-head drift marks this session for reopen but preserves the bounded
  /// compiler/recovery evidence in the returned receipt.
  Future<ManagedRevision3CompilerCheckReceipt> checkCompilerV1({
    required AuthoringRevision3ManagedCompilerEntityKind entityKind,
    required String gameRoot,
    required String entityId,
    required int expectedEntityRevision,
    required String expectedModuleId,
    required int expectedModuleRevision,
  }) async {
    final result = await _core
        .readBasisSnapshot<AuthoringRevision3ManagedCompilerCheckResult>(
          (basis) async {
            final selection = _managedRevision3CompilerSelection(
              currentProjectJson: basis.projectJson,
              entityKind: entityKind,
              entityId: entityId,
            );
            if (selection == null ||
                selection.entityRevision != expectedEntityRevision ||
                selection.moduleId != expectedModuleId ||
                selection.moduleRevision != expectedModuleRevision) {
              throw const ManagedRevision3CompilerSelectionStaleException();
            }
            final result = switch (entityKind) {
              AuthoringRevision3ManagedCompilerEntityKind.questDraft =>
                await _store.checkQuestCompilerV1(
                  root: root.path,
                  gameRoot: gameRoot,
                  expectedHead: basis.head,
                  questId: entityId,
                ),
              AuthoringRevision3ManagedCompilerEntityKind.npcDraft =>
                await _store.checkNpcCompilerV1(
                  root: root.path,
                  gameRoot: gameRoot,
                  expectedHead: basis.head,
                  npcId: entityId,
                ),
            };
            final projectBytes = utf8.encode(basis.projectJson);
            final projectId = basis.projectId;
            final projectRevision = basis.projectRevision;
            if (projectId == null ||
                projectRevision == null ||
                result.head.canonicalJson != basis.head.canonicalJson ||
                result.project.id != projectId ||
                result.project.revision != projectRevision ||
                result.project.seal.byteLength != projectBytes.length ||
                result.project.seal.sha256 !=
                    crypto.sha256.convert(projectBytes).toString() ||
                result.entity.kind != entityKind ||
                result.entity.id != entityId ||
                result.entity.revision != selection.entityRevision ||
                result.module.id != selection.moduleId ||
                result.module.revision != selection.moduleRevision ||
                result.module.namespace != selection.moduleNamespace ||
                result.module.relativePath != selection.moduleRelativePath ||
                result.module.sourceSha256 != selection.sourceSha256) {
              throw const ManagedProjectVerificationException(
                'revision-3 managed compiler evidence disagrees with its exact session basis',
              );
            }
            return result;
          },
          operation: 'checkCompilerV1',
          handleReadError: _core._throwRevision3ManagedCompilerCheckError,
        );
    return ManagedRevision3CompilerCheckReceipt(
      result: result,
      storeStillExactCurrent: !_core.requiresReopen,
    );
  }

  /// Read path-only installed DataAsset package candidates for the exact
  /// current project generation. Native code reopens both the Store and the
  /// selected installation, reads no ExportBundle payload, and returns no
  /// extraction, mutation, build, runtime, or publication authority.
  Future<AuthoringRevision3DataAssetPackageIndexResult>
  readDataAssetPackageIndexV1({
    required String gameRoot,
  }) => _core.readExact<AuthoringRevision3DataAssetPackageIndexResult>(
    (basis) async {
      final projectId = basis.projectId;
      final projectRevision = basis.projectRevision;
      if (projectId == null || projectRevision == null) {
        throw const ManagedProjectVerificationException(
          'revision-3 DataAsset package index has no exact project identity',
        );
      }
      final result = await _store.readDataAssetPackageIndexV1(
        root: root.path,
        gameRoot: gameRoot,
        expectedHead: basis.head,
      );
      if (result.head.canonicalJson != basis.head.canonicalJson ||
          result.projectId != projectId ||
          result.projectRevision != projectRevision ||
          !result.matchesCanonicalProjectTarget(basis.projectJson)) {
        throw const ManagedProjectVerificationException(
          'revision-3 DataAsset package index disagrees with its exact session basis',
        );
      }
      return result;
    },
    operation: 'readDataAssetPackageIndexV1',
    handleReadError: _core._throwRevision3DataAssetPackageIndexError,
  );

  /// Inspect one candidate selected by its original ordinal from an exact
  /// installed package snapshot. The native side rebuilds that snapshot,
  /// resolves the candidate itself, extracts only to bounded memory, and
  /// returns read-only fixed-leaf evidence. No path, project, or game file is
  /// written and this value grants no edit, build, runtime, or publication
  /// authority.
  Future<AuthoringRevision3InstalledDataAssetInspectionResult>
  inspectInstalledDataAssetV1({
    required String gameRoot,
    required AuthoringRevision3DataAssetPackageIndexResult expectedSnapshot,
    required AuthoringRevision3DataAssetPackageCandidate candidate,
  }) => _core.readExact<AuthoringRevision3InstalledDataAssetInspectionResult>(
    (basis) async {
      final projectId = basis.projectId;
      final projectRevision = basis.projectRevision;
      if (projectId == null || projectRevision == null) {
        throw const ManagedProjectVerificationException(
          'revision-3 installed DataAsset inspection has no exact project identity',
        );
      }
      if (expectedSnapshot.head.canonicalJson != basis.head.canonicalJson ||
          expectedSnapshot.projectId != projectId ||
          expectedSnapshot.projectRevision != projectRevision ||
          !expectedSnapshot.matchesCanonicalProjectTarget(basis.projectJson) ||
          candidate.ordinal < 0 ||
          candidate.ordinal >= expectedSnapshot.index.candidates.length ||
          !identical(
            candidate,
            expectedSnapshot.index.candidates[candidate.ordinal],
          )) {
        throw ArgumentError(
          'revision-3 installed DataAsset selection is not bound to the exact session basis',
          'expectedSnapshot',
        );
      }
      final result = await _store.inspectInstalledDataAssetV1(
        root: root.path,
        gameRoot: gameRoot,
        expectedHead: basis.head,
        expectedSnapshot: expectedSnapshot,
        candidate: candidate,
      );
      if (result.head.canonicalJson != basis.head.canonicalJson ||
          result.projectId != projectId ||
          result.projectRevision != projectRevision ||
          result.candidateOrdinal != candidate.ordinal ||
          result.targetPath != candidate.targetPath ||
          result.packageIdHex != candidate.packageIdHex ||
          !_sameRevision3DataAssetSeal(
            result.packageIndexSeal,
            expectedSnapshot.packageIndexSeal,
          ) ||
          !_sameRevision3DataAssetSeal(
            result.sourceSnapshotSeal,
            expectedSnapshot.sourceSnapshotSeal,
          )) {
        throw const ManagedProjectVerificationException(
          'revision-3 installed DataAsset inspection disagrees with its exact session basis',
        );
      }
      return result;
    },
    operation: 'inspectInstalledDataAssetV1',
    handleReadError: _core._throwRevision3InstalledDataAssetInspectionError,
  );

  /// Read the semantic content projection bound to the exact checkpoint owned by this session.
  ///
  /// The operation shares the session's serialized lane, verifies the fixed head before and after
  /// native projection, and never prepares objects or enters the publication path.
  Future<Revision3ContentIndex> readContentIndex() =>
      _core.readExact<Revision3ContentIndex>(
        (basis) async {
          final projectId = basis.projectId;
          final projectRevision = basis.projectRevision;
          if (projectId == null || projectRevision == null) {
            throw const ManagedProjectVerificationException(
              'revision-3 content read has no exact project identity',
            );
          }
          final result = await _store.readContentIndex(
            root: root.path,
            expectedHead: basis.head,
          );
          if (result.head.canonicalJson != basis.head.canonicalJson ||
              result.projectId != projectId ||
              result.projectRevision != projectRevision ||
              result.index.projectId != projectId ||
              result.index.projectRevision != projectRevision) {
            throw const ManagedProjectVerificationException(
              'revision-3 content read disagrees with its exact session basis',
            );
          }
          return result.index;
        },
        operation: 'readContentIndex',
        handleReadError: _core._throwRevision3ContentReadError,
      );

  /// Reopen the exact currently-published checkpoint with full asset
  /// verification without preparing or publishing a new checkpoint.
  Future<void> verifyCurrentHead() => _core.verifyCurrentHead();

  /// Repair and fully reopen an uncertain publication without releasing this
  /// session's exclusive project lock.
  Future<ManagedRevision3RecoveryCheckpoint>
  recoverAfterUncertainPublication() =>
      _core.recoverAfterUncertainPublication();

  Future<void> close() => _core.close();
}

/// Synchronous stale-selection preflight over an already fully-opened
/// canonical revision-3 project. The managed session repeats this derivation
/// inside its serialized exact-head lane before any native compiler call.
bool revision3ManagedCompilerSelectionMatches({
  required String currentProjectJson,
  required AuthoringRevision3ManagedCompilerEntityKind entityKind,
  required String entityId,
  required int expectedEntityRevision,
  required String expectedModuleId,
  required int expectedModuleRevision,
}) {
  final selection = _managedRevision3CompilerSelection(
    currentProjectJson: currentProjectJson,
    entityKind: entityKind,
    entityId: entityId,
  );
  return selection != null &&
      selection.entityRevision == expectedEntityRevision &&
      selection.moduleId == expectedModuleId &&
      selection.moduleRevision == expectedModuleRevision;
}

final class _ManagedRevision3CompilerSelection {
  const _ManagedRevision3CompilerSelection({
    required this.entityRevision,
    required this.moduleId,
    required this.moduleRevision,
    required this.moduleNamespace,
    required this.moduleRelativePath,
    required this.sourceSha256,
  });

  final int entityRevision;
  final String moduleId;
  final int moduleRevision;
  final String moduleNamespace;
  final String moduleRelativePath;
  final String sourceSha256;
}

_ManagedRevision3CompilerSelection? _managedRevision3CompilerSelection({
  required String currentProjectJson,
  required AuthoringRevision3ManagedCompilerEntityKind entityKind,
  required String entityId,
}) {
  if (!_managedRevision3CompilerIdPattern.hasMatch(entityId) ||
      entityId == _managedRevision3CompilerZeroId) {
    return null;
  }
  final Object? decoded;
  try {
    decoded = jsonDecode(currentProjectJson);
  } on FormatException catch (error) {
    throw ManagedProjectVerificationException(
      'canonical revision-3 compiler basis is not JSON: ${error.message}',
    );
  }
  final project = _managedRevision3CompilerObject(decoded, 'project');
  final projectId = _managedRevision3CompilerId(
    project['project_id'],
    'project_id',
  );
  _managedRevision3CompilerRevision(project['revision'], 'project revision');
  final entities = _managedRevision3CompilerObject(
    project['entities'],
    'project entities',
  );
  final rawEntity = entities[entityId];
  if (rawEntity == null) return null;
  final entity = _managedRevision3CompilerObject(
    rawEntity,
    'selected compiler entity',
  );
  if (_managedRevision3CompilerId(entity['id'], 'selected entity id') !=
      entityId) {
    throw const ManagedProjectVerificationException(
      'selected compiler entity key and identity disagree',
    );
  }
  final payload = _managedRevision3CompilerObject(
    entity['payload'],
    'selected compiler entity payload',
  );
  if (payload['kind'] != entityKind.wireName) return null;
  final entityRevision = _managedRevision3CompilerRevision(
    entity['revision'],
    'selected entity revision',
  );
  final entityData = _managedRevision3CompilerObject(
    payload['data'],
    'selected compiler entity data',
  );
  final moduleReference = _managedRevision3CompilerReference(
    entityData['script_module'],
    context: 'selected compiler ScriptModule reference',
    expectedProjectId: projectId,
    expectedKind: 'script_module',
  );
  if (moduleReference.id == entityId) {
    throw const ManagedProjectVerificationException(
      'selected compiler entity aliases its ScriptModule',
    );
  }
  final rawModule = entities[moduleReference.id];
  if (rawModule == null) {
    throw const ManagedProjectVerificationException(
      'selected compiler ScriptModule is absent from the exact project',
    );
  }
  final module = _managedRevision3CompilerObject(
    rawModule,
    'selected compiler ScriptModule',
  );
  if (_managedRevision3CompilerId(module['id'], 'ScriptModule id') !=
      moduleReference.id) {
    throw const ManagedProjectVerificationException(
      'selected compiler ScriptModule key and identity disagree',
    );
  }
  final moduleRevision = _managedRevision3CompilerRevision(
    module['revision'],
    'ScriptModule revision',
  );
  final modulePayload = _managedRevision3CompilerObject(
    module['payload'],
    'selected compiler ScriptModule payload',
  );
  if (modulePayload['kind'] != 'script_module') {
    throw const ManagedProjectVerificationException(
      'selected compiler module reference targets another entity kind',
    );
  }
  final moduleData = _managedRevision3CompilerObject(
    modulePayload['data'],
    'selected compiler ScriptModule data',
  );
  final owner = _managedRevision3CompilerReference(
    moduleData['owner'],
    context: 'selected compiler ScriptModule owner',
    expectedProjectId: projectId,
    expectedKind: entityKind.wireName,
  );
  if (owner.id != entityId) {
    throw const ManagedProjectVerificationException(
      'selected compiler ScriptModule belongs to another entity',
    );
  }
  final moduleNamespace = _managedRevision3CompilerString(
    moduleData['module_namespace'],
    'ScriptModule namespace',
  );
  final moduleRelativePath = _managedRevision3CompilerString(
    moduleData['module_relative_path'],
    'ScriptModule relative path',
  );
  if (moduleRelativePath != '${moduleNamespace.replaceAll('.', '/')}.as') {
    throw const ManagedProjectVerificationException(
      'selected compiler ScriptModule namespace and path disagree',
    );
  }
  final sourceSha256 = _managedRevision3CompilerString(
    moduleData['source_sha256'],
    'ScriptModule source SHA-256',
  );
  if (!_managedRevision3CompilerShaPattern.hasMatch(sourceSha256)) {
    throw const ManagedProjectVerificationException(
      'selected compiler ScriptModule source SHA-256 is invalid',
    );
  }
  return _ManagedRevision3CompilerSelection(
    entityRevision: entityRevision,
    moduleId: moduleReference.id,
    moduleRevision: moduleRevision,
    moduleNamespace: moduleNamespace,
    moduleRelativePath: moduleRelativePath,
    sourceSha256: sourceSha256,
  );
}

const _managedRevision3CompilerZeroId = '00000000000000000000000000000000';
final _managedRevision3CompilerIdPattern = RegExp(r'^[0-9a-f]{32}$');
final _managedRevision3CompilerShaPattern = RegExp(r'^[0-9a-f]{64}$');

({String id}) _managedRevision3CompilerReference(
  Object? value, {
  required String context,
  required String expectedProjectId,
  required String expectedKind,
}) {
  final reference = _managedRevision3CompilerObject(value, context);
  if (_managedRevision3CompilerId(
        reference['project_id'],
        '$context project',
      ) !=
      expectedProjectId) {
    throw ManagedProjectVerificationException(
      '$context belongs to another project',
    );
  }
  if (reference['expected_kind'] != expectedKind) {
    throw ManagedProjectVerificationException(
      '$context names another entity kind',
    );
  }
  return (id: _managedRevision3CompilerId(reference['id'], '$context id'));
}

Map<String, Object?> _managedRevision3CompilerObject(
  Object? value,
  String context,
) {
  if (value is! Map) {
    throw ManagedProjectVerificationException('$context is not an object');
  }
  final result = <String, Object?>{};
  for (final entry in value.entries) {
    if (entry.key is! String) {
      throw ManagedProjectVerificationException(
        '$context has a non-string field',
      );
    }
    result[entry.key as String] = entry.value;
  }
  return result;
}

String _managedRevision3CompilerId(Object? value, String context) {
  if (value is! String ||
      !_managedRevision3CompilerIdPattern.hasMatch(value) ||
      value == _managedRevision3CompilerZeroId) {
    throw ManagedProjectVerificationException('$context is not a valid ID');
  }
  return value;
}

int _managedRevision3CompilerRevision(Object? value, String context) {
  if (value is! int || value < 0 || value > 0x7fffffffffffffff) {
    throw ManagedProjectVerificationException(
      '$context is outside the signed wire domain',
    );
  }
  return value;
}

String _managedRevision3CompilerString(Object? value, String context) {
  if (value is! String || value.isEmpty || value.contains('\u0000')) {
    throw ManagedProjectVerificationException(
      '$context is not bounded non-empty text',
    );
  }
  return value;
}

bool _sameOrderedStrings(List<String> left, List<String> right) {
  if (left.length != right.length) return false;
  for (var index = 0; index < left.length; index++) {
    if (left[index] != right[index]) return false;
  }
  return true;
}

class _ManagedProjectSessionCore {
  _ManagedProjectSessionCore._({
    required this.root,
    required this._store,
    required this._lock,
    required this._replacement,
    required this._opened,
  });

  final Directory root;
  final _ManagedCheckpointStore _store;
  final ManagedProjectSessionLock _lock;
  final AtomicByteReplacement _replacement;

  _ManagedOpenedCheckpoint _opened;
  Future<void> _tail = Future<void>.value();
  Future<void>? _closeFuture;
  bool _closeRequested = false;
  bool _closed = false;
  bool _requiresReopen = false;
  final Object _deriveZoneKey = Object();

  String get projectJson => _opened.projectJson;
  AuthoringWorkingHead get head => _opened.head;
  bool get isClosed => _closed;
  bool get supportsReviewedDataAssetBuild =>
      _store.supportsReviewedDataAssetBuild;

  /// True after an I/O or verification failure leaves publication state
  /// uncertain. Recover when supported, or close and reopen before editing.
  bool get requiresReopen => _requiresReopen;

  void markRequiresReopenAfterPublicationUncertainty() {
    _requiresReopen = true;
  }

  File get headFile => File(p.join(root.path, 'gore-project.json'));

  static Future<_ManagedProjectSessionCore> create({
    required Directory root,
    required _ManagedCheckpointStore store,
    required String projectJson,
    AtomicByteReplacement? replacement,
  }) async {
    final lock = await ManagedProjectSessionLock.acquire(root);
    final normalizedRoot = Directory(lock.projectRoot);
    final byteReplacement = replacement ?? AtomicByteReplacement();
    try {
      final operations = _ManagedSessionOperations(
        root: normalizedRoot,
        store: store,
        replacement: byteReplacement,
      );
      final headType = await FileSystemEntity.type(
        operations.headFile.path,
        followLinks: false,
      );
      final journalType = await FileSystemEntity.type(
        AtomicByteReplacement.journalPathFor(operations.headFile),
        followLinks: false,
      );
      if (headType != FileSystemEntityType.notFound ||
          journalType != FileSystemEntityType.notFound) {
        throw ManagedProjectAlreadyInitializedException(
          'managed project already has a head or pending recovery journal: '
          '${operations.headFile.path}',
        );
      }
      // A create operation must never select or publish a generation from a
      // pre-existing fixed journal. With both fixed artifacts absent, repair
      // can only discard journal staging that predates any content mutation.
      await operations.repairHead();

      final preparedHead = await store.prepareCheckpoint(
        root: normalizedRoot.path,
        expectedHead: null,
        projectJson: projectJson,
      );
      await operations.verifyPreparedCheckpoint(
        preparedHead,
        expectedProjectJson: projectJson,
      );
      await operations.publish(preparedHead, expectedHead: null);
      final opened = await operations.openPublished(
        expectedHead: preparedHead,
        expectedProjectJson: projectJson,
      );
      return _ManagedProjectSessionCore._(
        root: normalizedRoot,
        store: store,
        lock: lock,
        replacement: byteReplacement,
        opened: opened,
      );
    } catch (error, stackTrace) {
      try {
        await lock.release();
      } catch (_) {}
      Error.throwWithStackTrace(error, stackTrace);
    }
  }

  static Future<_ManagedProjectSessionCore> open({
    required Directory root,
    required _ManagedCheckpointStore store,
    AtomicByteReplacement? replacement,
  }) async {
    final lock = await ManagedProjectSessionLock.acquire(root);
    final normalizedRoot = Directory(lock.projectRoot);
    final byteReplacement = replacement ?? AtomicByteReplacement();
    try {
      final operations = _ManagedSessionOperations(
        root: normalizedRoot,
        store: store,
        replacement: byteReplacement,
      );
      await operations.repairHead();
      final opened = await operations.openPublished();
      return _ManagedProjectSessionCore._(
        root: normalizedRoot,
        store: store,
        lock: lock,
        replacement: byteReplacement,
        opened: opened,
      );
    } catch (error, stackTrace) {
      try {
        await lock.release();
      } catch (_) {}
      Error.throwWithStackTrace(error, stackTrace);
    }
  }

  /// Save a captured canonical format-2 document in invocation order.
  Future<void> save(String projectJson) {
    if (_isActiveDeriveCallbackZone) {
      return _reentrantOperation<void>('save');
    }
    if (_closeRequested) {
      return Future<void>.error(
        const ManagedProjectSessionClosedException(
          'managed project session is closing or closed',
        ),
      );
    }
    final capturedProjectJson = projectJson;
    return _enqueue(() => _saveCapturedInQueue(capturedProjectJson));
  }

  /// Derive from the exact project current when this invocation reaches the serialized session
  /// lane. A rejection returns without any store or filesystem write. A candidate reuses the
  /// complete verified save pipeline before its value becomes visible to the caller.
  ///
  /// The callback must not re-enter this same session: it already owns the operation lane.
  Future<T> deriveAndSave<T>(ManagedProjectDeriver<T> derive) {
    if (_isActiveDeriveCallbackZone) {
      return _reentrantOperation<T>('deriveAndSave');
    }
    return _enqueue(() async {
      _requireWritableState();
      final exactHead = _opened.head;
      final operations = _ManagedSessionOperations(
        root: root,
        store: _store,
        replacement: _replacement,
      );
      await _requireExactPublishedHead(operations, exactHead);

      final callbackToken = _ManagedProjectDeriveZoneToken();
      final ManagedProjectDerivedSave<T> decision;
      try {
        decision = await runZoned(
          () => Future<ManagedProjectDerivedSave<T>>.sync(
            () => derive(_opened.projectJson),
          ),
          zoneValues: <Object, Object>{_deriveZoneKey: callbackToken},
        );
      } catch (error, stackTrace) {
        // A failed callback still observed this exact published head. If it drifted while the
        // callback was suspended, surface and poison that stronger session-integrity failure;
        // otherwise preserve the callback's original error and stack.
        await _requireExactPublishedHead(operations, exactHead);
        Error.throwWithStackTrace(error, stackTrace);
      } finally {
        callbackToken.active = false;
      }
      switch (decision) {
        case ManagedProjectDerivedRejection<T> rejection:
          await _requireExactPublishedHead(operations, exactHead);
          return rejection.value;
        case ManagedProjectDerivedCandidate<T> candidate:
          await _saveCapturedInQueue(candidate.projectJson);
          return candidate.value;
      }
    });
  }

  /// Execute one read against the exact current checkpoint without mutation.
  ///
  /// The fixed head is checked on both sides of the awaited native read. Integrity,
  /// response-shape, or store failures poison the session; bounded semantic/read-capacity and
  /// unavailable-transport failures remain retryable when the exact disk head is unchanged.
  Future<T> readExact<T>(
    Future<T> Function(_ManagedOpenedCheckpoint basis) read, {
    required String operation,
    required Never Function(Object error, StackTrace stackTrace)
    handleReadError,
  }) {
    if (_isActiveDeriveCallbackZone) {
      return _reentrantOperation<T>(operation);
    }
    return _enqueue(() async {
      _requireWritableState();
      final basis = _opened;
      final operations = _ManagedSessionOperations(
        root: root,
        store: _store,
        replacement: _replacement,
      );
      await _requireExactPublishedHead(operations, basis.head);
      final T result;
      try {
        result = await read(basis);
      } catch (error, stackTrace) {
        // If native work raced an external head write, that drift is the stronger failure.
        await _requireExactPublishedHead(operations, basis.head);
        handleReadError(error, stackTrace);
      }
      await _requireExactPublishedHead(operations, basis.head);
      return result;
    });
  }

  /// Resolve whether an ambiguous History Store failure belongs to the exact
  /// current checkpoint or only to an older retained checkpoint.
  ///
  /// The History command can report the same Store error code for either
  /// location. A complete exact-current reopen is therefore required before a
  /// retained-History failure may be treated as capability-local. Failure or
  /// drift here poisons the session; success leaves [_opened] unchanged.
  Future<void> _reverifyExactCurrentAfterHistoryFailure(
    _ManagedOpenedCheckpoint basis,
  ) async {
    final operations = _ManagedSessionOperations(
      root: root,
      store: _store,
      replacement: _replacement,
    );
    try {
      await operations.openPublished(
        expectedHead: basis.head,
        expectedProjectJson: basis.projectJson,
      );
    } catch (error, stackTrace) {
      _requiresReopen = true;
      if (error is ManagedProjectSessionException) {
        Error.throwWithStackTrace(error, stackTrace);
      }
      Error.throwWithStackTrace(
        const ManagedProjectVerificationException(
          'managed revision-3 current checkpoint failed full verification after a History read error',
        ),
        stackTrace,
      );
    }
  }

  /// Execute one immutable-artifact read against the exact basis checkpoint.
  ///
  /// The published head must match before native work starts. Once native code
  /// returns a fully verified basis-bound artifact receipt, a later independent
  /// head publication cannot invalidate that already-created output. A failed
  /// post-read head audit therefore marks this session for reopen but preserves
  /// the receipt so callers can identify and safely manage the output path.
  Future<T> readBasisSnapshot<T>(
    Future<T> Function(_ManagedOpenedCheckpoint basis) read, {
    required String operation,
    required Never Function(Object error, StackTrace stackTrace)
    handleReadError,
  }) {
    if (_isActiveDeriveCallbackZone) {
      return _reentrantOperation<T>(operation);
    }
    return _enqueue(() async {
      _requireWritableState();
      final basis = _opened;
      final operations = _ManagedSessionOperations(
        root: root,
        store: _store,
        replacement: _replacement,
      );
      await _requireExactPublishedHead(operations, basis.head);
      final T result;
      try {
        result = await read(basis);
      } catch (error, stackTrace) {
        // No valid receipt was returned. Preserve readExact's fail-closed error
        // classification and surface any stronger concurrent head drift.
        await _requireExactPublishedHead(operations, basis.head);
        handleReadError(error, stackTrace);
      }
      try {
        await operations.requirePublishedHead(basis.head);
      } catch (_) {
        // The artifact is sealed to [basis], but this lease can no longer make
        // another exact-current claim until the project is reopened.
        _requiresReopen = true;
      }
      return result;
    });
  }

  /// Publish an already-prepared immutable candidate through the same exact-head lane as save.
  ///
  /// The callback receives the exact fully-opened basis only inside the serialized lane. It may
  /// install immutable CAS objects, but it must not touch the fixed head. Its candidate is fully
  /// reopened here before any publication is attempted.
  Future<T> _publishPreparedRevision3Checkpoint<T>({
    required String operation,
    required Future<_ManagedPreparedCheckpoint<T>> Function(
      _ManagedOpenedCheckpoint basis,
    )
    prepare,
    required Never Function(Object error, StackTrace stackTrace)
    handlePrepareError,
  }) {
    if (_isActiveDeriveCallbackZone) {
      return _reentrantOperation<T>(operation);
    }
    return _enqueue(() async {
      _requireWritableState();
      final basis = _opened;
      final operations = _ManagedSessionOperations(
        root: root,
        store: _store,
        replacement: _replacement,
      );
      await _requireExactPublishedHead(operations, basis.head);

      final _ManagedPreparedCheckpoint<T> prepared;
      try {
        prepared = await prepare(basis);
      } catch (error, stackTrace) {
        // A native prepare can suspend for a long game/catalog rebuild. A concurrent head drift is
        // the stronger integrity failure and must poison the session even when preparation also
        // reports a semantic or transport error.
        await _requireExactPublishedHead(operations, basis.head);
        handlePrepareError(error, stackTrace);
      }

      await _requireExactPublishedHead(operations, basis.head);
      try {
        await operations.verifyPreparedCheckpoint(
          prepared.head,
          expectedProjectJson: prepared.projectJson,
        );
      } catch (error, stackTrace) {
        await _requireExactPublishedHead(operations, basis.head);
        Error.throwWithStackTrace(error, stackTrace);
      }

      try {
        await operations.publish(prepared.head, expectedHead: basis.head);
      } on AtomicSwapConflictException catch (error) {
        _requiresReopen = true;
        throw ManagedProjectHeadConflictException(error.message);
      } on AtomicSwapException {
        _requiresReopen = true;
        rethrow;
      } catch (_) {
        _requiresReopen = true;
        rethrow;
      }

      try {
        _opened = await operations.openPublished(
          expectedHead: prepared.head,
          expectedProjectJson: prepared.projectJson,
        );
      } catch (_) {
        _requiresReopen = true;
        rethrow;
      }
      return prepared.value;
    });
  }

  /// Verify and fully reopen the exact head currently owned by this session.
  ///
  /// This is a durability check, not a save: it prepares no immutable objects
  /// and never enters the publication lane. Any drift or reopen failure poisons
  /// the session until verified recovery or a close and reopen.
  Future<void> verifyCurrentHead() {
    if (_isActiveDeriveCallbackZone) {
      return _reentrantOperation<void>('verifyCurrentHead');
    }
    return _enqueue(() async {
      _requireWritableState();
      final exactOpened = _opened;
      final operations = _ManagedSessionOperations(
        root: root,
        store: _store,
        replacement: _replacement,
      );
      await _requireExactPublishedHead(operations, exactOpened.head);
      try {
        _opened = await operations.openPublished(
          expectedHead: exactOpened.head,
          expectedProjectJson: exactOpened.projectJson,
        );
      } catch (_) {
        _requiresReopen = true;
        rethrow;
      }
    });
  }

  /// Repair an interrupted fixed-head publication while retaining this
  /// session's operation lane and OS lock. The old checkpoint remains visible
  /// and the poison latch remains set until the repaired generation has passed
  /// a full Store reopen and every closed recovery invariant.
  Future<ManagedRevision3RecoveryCheckpoint>
  recoverAfterUncertainPublication() {
    if (_isActiveDeriveCallbackZone) {
      return _reentrantOperation<ManagedRevision3RecoveryCheckpoint>(
        'recoverAfterUncertainPublication',
      );
    }
    return _enqueue(() async {
      if (!_requiresReopen) {
        throw const ManagedProjectVerificationException(
          'managed project recovery requires an uncertain publication',
        );
      }

      final previous = _opened;
      final previousIdentity = _revision3RecoveryIdentity(
        previous,
        context: 'previous revision-3 recovery checkpoint',
      );
      final operations = _ManagedSessionOperations(
        root: root,
        store: _store,
        replacement: _replacement,
      );

      // Recovery deliberately bypasses the ordinary writable-state guard, but
      // never the poison latch. Any failure below leaves both the old in-memory
      // checkpoint and the recovery requirement intact.
      _requiresReopen = true;
      try {
        final repairOutcome = await operations.repairHead();
        final recovered = await operations.openPublished();
        final recoveredIdentity = _revision3RecoveryIdentity(
          recovered,
          context: 'recovered revision-3 checkpoint',
        );
        _requireValidRevision3Recovery(
          previous: previous,
          previousIdentity: previousIdentity,
          recovered: recovered,
          recoveredIdentity: recoveredIdentity,
        );

        final checkpoint = ManagedRevision3RecoveryCheckpoint(
          previousHead: previous.head,
          recoveredHead: recovered.head,
          projectId: recoveredIdentity.projectId,
          previousProjectRevision: previousIdentity.revision,
          recoveredProjectRevision: recoveredIdentity.revision,
          repairOutcome: repairOutcome,
          canonicalProjectJson: recovered.projectJson,
        );
        _opened = recovered;
        _requiresReopen = false;
        return checkpoint;
      } catch (error, stackTrace) {
        _requiresReopen = true;
        Error.throwWithStackTrace(error, stackTrace);
      }
    });
  }

  bool get _isActiveDeriveCallbackZone {
    final token = Zone.current[_deriveZoneKey];
    return token is _ManagedProjectDeriveZoneToken && token.active;
  }

  Future<T> _reentrantOperation<T>(String operation) => Future<T>.error(
    ManagedProjectReentrantOperationException(
      'managed project $operation cannot be called from its active derive callback',
    ),
  );

  void _requireWritableState() {
    if (_requiresReopen) {
      throw const ManagedProjectVerificationException(
        'managed project requires recovery after an uncertain publication',
      );
    }
  }

  Never _throwRevision3QuestPrepareError(Object error, StackTrace stackTrace) {
    if (error is ModFfiException) {
      if (error.code == 'AUTHORING_REVISION3_QUEST_HEAD_CONFLICT') {
        _requiresReopen = true;
        Error.throwWithStackTrace(
          ManagedProjectHeadConflictException(error.message),
          stackTrace,
        );
      }
      if (_revision3QuestPrepareErrorIsRetryable(error.code)) {
        Error.throwWithStackTrace(error, stackTrace);
      }
      // Every integrity code, malformed native response, and future unknown
      // code fails closed until it is deliberately classified.
      _requiresReopen = true;
      Error.throwWithStackTrace(
        ManagedProjectVerificationException(error.message),
        stackTrace,
      );
    }
    if (error is ArgumentError || error is FormatException) {
      // Local request construction fails before native work begins. Production
      // native response-shape failures use MALFORMED_NATIVE_RESPONSE instead.
      Error.throwWithStackTrace(error, stackTrace);
    }
    _requiresReopen = true;
    if (error is ManagedProjectSessionException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    Error.throwWithStackTrace(
      const ManagedProjectVerificationException(
        'managed revision-3 Quest preparation could not be verified exactly',
      ),
      stackTrace,
    );
  }

  Never _throwRevision3QuestOutlinePrepareError(
    Object error,
    StackTrace stackTrace,
  ) {
    if (error is ModFfiException) {
      if (error.code == 'AUTHORING_REVISION3_QUEST_OUTLINE_HEAD_CONFLICT' ||
          error.code == 'AUTHORING_REVISION3_QUEST_OUTLINE_V2_HEAD_CONFLICT') {
        _requiresReopen = true;
        Error.throwWithStackTrace(
          ManagedProjectHeadConflictException(error.message),
          stackTrace,
        );
      }
      if (_revision3QuestOutlinePrepareErrorIsRetryable(error.code)) {
        // The exact fixed head was rechecked after native preparation. These
        // failures are bounded semantic/capacity rejections and can leave at
        // most immutable CAS orphans, never an uncertain publication.
        Error.throwWithStackTrace(error, stackTrace);
      }
      _requiresReopen = true;
      Error.throwWithStackTrace(
        ManagedProjectVerificationException(error.message),
        stackTrace,
      );
    }
    if (error is ArgumentError || error is FormatException) {
      // Local construction failed before native work. Native response-shape
      // failures are wrapped as MALFORMED_NATIVE_RESPONSE and poison above.
      Error.throwWithStackTrace(error, stackTrace);
    }
    _requiresReopen = true;
    if (error is ManagedProjectSessionException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    Error.throwWithStackTrace(
      const ManagedProjectVerificationException(
        'managed revision-3 Quest outline preparation could not be verified exactly',
      ),
      stackTrace,
    );
  }

  Never _throwRevision3QuestTranscriptPrepareError(
    Object error,
    StackTrace stackTrace,
  ) {
    if (error is ModFfiException) {
      if (error.code == 'AUTHORING_REVISION3_QUEST_TRANSCRIPT_HEAD_CONFLICT') {
        _requiresReopen = true;
        Error.throwWithStackTrace(
          ManagedProjectHeadConflictException(error.message),
          stackTrace,
        );
      }
      if (_revision3QuestTranscriptPrepareErrorIsRetryable(error.code)) {
        Error.throwWithStackTrace(error, stackTrace);
      }
      _requiresReopen = true;
      Error.throwWithStackTrace(
        ManagedProjectVerificationException(error.message),
        stackTrace,
      );
    }
    if (error is ArgumentError || error is FormatException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    _requiresReopen = true;
    if (error is ManagedProjectSessionException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    Error.throwWithStackTrace(
      const ManagedProjectVerificationException(
        'managed revision-3 Quest transcript preparation could not be verified exactly',
      ),
      stackTrace,
    );
  }

  Never _throwRevision3NpcGreetingPrepareError(
    Object error,
    StackTrace stackTrace,
  ) {
    if (error is ModFfiException) {
      if (error.code == 'AUTHORING_REVISION3_NPC_GREETING_HEAD_CONFLICT') {
        _requiresReopen = true;
        Error.throwWithStackTrace(
          ManagedProjectHeadConflictException(error.message),
          stackTrace,
        );
      }
      if (_revision3NpcGreetingPrepareErrorIsRetryable(error.code)) {
        Error.throwWithStackTrace(error, stackTrace);
      }
      _requiresReopen = true;
      Error.throwWithStackTrace(
        ManagedProjectVerificationException(error.message),
        stackTrace,
      );
    }
    if (error is ArgumentError || error is FormatException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    _requiresReopen = true;
    if (error is ManagedProjectSessionException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    Error.throwWithStackTrace(
      const ManagedProjectVerificationException(
        'managed revision-3 NPC greeting preparation could not be verified exactly',
      ),
      stackTrace,
    );
  }

  Never _throwRevision3QuestContextPrepareError(
    Object error,
    StackTrace stackTrace,
  ) {
    if (error is ModFfiException) {
      if (error.code == 'AUTHORING_REVISION3_QUEST_CONTEXT_HEAD_CONFLICT') {
        _requiresReopen = true;
        Error.throwWithStackTrace(
          ManagedProjectHeadConflictException(error.message),
          stackTrace,
        );
      }
      if (_revision3QuestContextPrepareErrorIsRetryable(error.code)) {
        Error.throwWithStackTrace(error, stackTrace);
      }
      _requiresReopen = true;
      Error.throwWithStackTrace(
        ManagedProjectVerificationException(error.message),
        stackTrace,
      );
    }
    if (error is ArgumentError || error is FormatException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    _requiresReopen = true;
    if (error is ManagedProjectSessionException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    Error.throwWithStackTrace(
      const ManagedProjectVerificationException(
        'managed revision-3 Quest context preparation could not be verified exactly',
      ),
      stackTrace,
    );
  }

  Never _throwRevision3QuestTransitionsPrepareError(
    Object error,
    StackTrace stackTrace,
  ) {
    if (error is ModFfiException) {
      if (error.code == 'AUTHORING_REVISION3_QUEST_TRANSITIONS_HEAD_CONFLICT') {
        _requiresReopen = true;
        Error.throwWithStackTrace(
          ManagedProjectHeadConflictException(error.message),
          stackTrace,
        );
      }
      if (_revision3QuestTransitionsPrepareErrorIsRetryable(error.code)) {
        Error.throwWithStackTrace(error, stackTrace);
      }
      _requiresReopen = true;
      Error.throwWithStackTrace(
        ManagedProjectVerificationException(error.message),
        stackTrace,
      );
    }
    if (error is ArgumentError || error is FormatException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    _requiresReopen = true;
    if (error is ManagedProjectSessionException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    Error.throwWithStackTrace(
      const ManagedProjectVerificationException(
        'managed revision-3 Quest transitions preparation could not be verified exactly',
      ),
      stackTrace,
    );
  }

  Never _throwRevision3QuestTransitionsSeedReadError(
    Object error,
    StackTrace stackTrace,
  ) {
    if (error is ArgumentError || error is FormatException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    _requiresReopen = true;
    if (error is ManagedProjectSessionException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    Error.throwWithStackTrace(
      const ManagedProjectVerificationException(
        'managed revision-3 Quest transitions seed could not be verified exactly',
      ),
      stackTrace,
    );
  }

  Never _throwRevision3QuestContextSeedReadError(
    Object error,
    StackTrace stackTrace,
  ) {
    // A selected ContentIndex entity can be stale without making the already
    // reopened project uncertain. The coordinator maps this local mismatch to
    // a close-and-reopen-editor result.
    if (error is ArgumentError || error is FormatException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    _requiresReopen = true;
    if (error is ManagedProjectSessionException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    Error.throwWithStackTrace(
      const ManagedProjectVerificationException(
        'managed revision-3 Quest context seed could not be verified exactly',
      ),
      stackTrace,
    );
  }

  Never _throwRevision3QuestSourceInspectionError(
    Object error,
    StackTrace stackTrace,
  ) {
    if (error is ModFfiException) {
      if (error.code == 'AUTHORING_REVISION3_QUEST_INSPECTION_HEAD_CONFLICT') {
        _requiresReopen = true;
        Error.throwWithStackTrace(
          ManagedProjectHeadConflictException(error.message),
          stackTrace,
        );
      }
      if (_revision3QuestSourceInspectionErrorIsRetryable(error.code)) {
        Error.throwWithStackTrace(error, stackTrace);
      }
      _requiresReopen = true;
      Error.throwWithStackTrace(
        ManagedProjectVerificationException(error.message),
        stackTrace,
      );
    }
    if (error is ArgumentError || error is FormatException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    _requiresReopen = true;
    if (error is ManagedProjectSessionException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    Error.throwWithStackTrace(
      const ManagedProjectVerificationException(
        'managed revision-3 Quest source inspection could not be verified exactly',
      ),
      stackTrace,
    );
  }

  Never _throwRevision3NpcSourceInspectionError(
    Object error,
    StackTrace stackTrace,
  ) {
    if (error is ModFfiException) {
      if (error.code == 'AUTHORING_REVISION3_NPC_INSPECTION_HEAD_CONFLICT') {
        _requiresReopen = true;
        Error.throwWithStackTrace(
          ManagedProjectHeadConflictException(error.message),
          stackTrace,
        );
      }
      if (_revision3NpcSourceInspectionErrorIsRetryable(error.code)) {
        Error.throwWithStackTrace(error, stackTrace);
      }
      _requiresReopen = true;
      Error.throwWithStackTrace(
        ManagedProjectVerificationException(error.message),
        stackTrace,
      );
    }
    if (error is ArgumentError || error is FormatException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    _requiresReopen = true;
    if (error is ManagedProjectSessionException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    Error.throwWithStackTrace(
      const ManagedProjectVerificationException(
        'managed revision-3 NPC source inspection could not be verified exactly',
      ),
      stackTrace,
    );
  }

  Never _throwRevision3ManagedCompilerCheckError(
    Object error,
    StackTrace stackTrace,
  ) {
    if (error is ManagedRevision3CompilerSelectionStaleException ||
        error is ArgumentError ||
        error is FormatException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    if (error is ModFfiException) {
      if (error.code == 'AUTHORING_REVISION3_COMPILER_HEAD_CONFLICT') {
        _requiresReopen = true;
        Error.throwWithStackTrace(
          ManagedProjectHeadConflictException(error.message),
          stackTrace,
        );
      }
      if (_revision3ManagedCompilerCheckErrorIsRetryable(error.code)) {
        Error.throwWithStackTrace(error, stackTrace);
      }
      // Compiler rejection, diagnostics, restore disposition, and install
      // recovery are valid result values. A thrown native error therefore has
      // no trustworthy evidence boundary and fails closed until deliberately
      // classified otherwise.
      _requiresReopen = true;
      Error.throwWithStackTrace(
        ManagedProjectVerificationException(error.message),
        stackTrace,
      );
    }
    _requiresReopen = true;
    if (error is ManagedProjectSessionException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    Error.throwWithStackTrace(
      const ManagedProjectVerificationException(
        'managed revision-3 compiler evidence could not be verified exactly',
      ),
      stackTrace,
    );
  }

  Never _throwRevision3DataAssetPackageIndexError(
    Object error,
    StackTrace stackTrace,
  ) {
    if (error is ModFfiException) {
      if (error.code ==
          'AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_HEAD_CONFLICT') {
        _requiresReopen = true;
        Error.throwWithStackTrace(
          ManagedProjectHeadConflictException(error.message),
          stackTrace,
        );
      }
      if (_revision3DataAssetPackageIndexErrorIsRetryable(error.code)) {
        Error.throwWithStackTrace(error, stackTrace);
      }
      _requiresReopen = true;
      Error.throwWithStackTrace(
        ManagedProjectVerificationException(error.message),
        stackTrace,
      );
    }
    if (error is ArgumentError || error is FormatException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    _requiresReopen = true;
    if (error is ManagedProjectSessionException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    Error.throwWithStackTrace(
      const ManagedProjectVerificationException(
        'managed revision-3 DataAsset package index could not be verified exactly',
      ),
      stackTrace,
    );
  }

  Never _throwRevision3InstalledDataAssetInspectionError(
    Object error,
    StackTrace stackTrace,
  ) {
    if (error is ModFfiException) {
      if (error.code ==
          'AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_HEAD_CONFLICT') {
        _requiresReopen = true;
        Error.throwWithStackTrace(
          ManagedProjectHeadConflictException(error.message),
          stackTrace,
        );
      }
      if (_revision3InstalledDataAssetInspectionErrorIsRetryable(error.code)) {
        Error.throwWithStackTrace(error, stackTrace);
      }
      _requiresReopen = true;
      Error.throwWithStackTrace(
        ManagedProjectVerificationException(error.message),
        stackTrace,
      );
    }
    if (error is ArgumentError || error is FormatException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    _requiresReopen = true;
    if (error is ManagedProjectSessionException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    Error.throwWithStackTrace(
      const ManagedProjectVerificationException(
        'managed revision-3 installed DataAsset inspection could not be verified exactly',
      ),
      stackTrace,
    );
  }

  Never _throwRevision3NpcPrepareError(Object error, StackTrace stackTrace) {
    if (error is ModFfiException) {
      if (error.code == 'AUTHORING_REVISION3_NPC_HEAD_CONFLICT') {
        _requiresReopen = true;
        Error.throwWithStackTrace(
          ManagedProjectHeadConflictException(error.message),
          stackTrace,
        );
      }
      if (_revision3NpcPrepareErrorIsRetryable(error.code)) {
        // Selection, collision, capacity, game-input, and unsupported-generation errors are
        // retryable after the caller has rechecked the exact fixed head around native work.
        // Native preparation can leave only immutable CAS orphans.
        Error.throwWithStackTrace(error, stackTrace);
      }
      // Fail closed for every integrity code and every future/unknown native code. A newly added
      // native failure must be classified deliberately before this session may retry it.
      _requiresReopen = true;
      Error.throwWithStackTrace(
        ManagedProjectVerificationException(error.message),
        stackTrace,
      );
    }
    if (error is ArgumentError || error is FormatException) {
      // These arise only while locally constructing the typed request before calling native code.
      // Native response-shape failures are wrapped as MALFORMED_NATIVE_RESPONSE by ModFfi.
      Error.throwWithStackTrace(error, stackTrace);
    }
    _requiresReopen = true;
    if (error is ManagedProjectSessionException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    Error.throwWithStackTrace(
      const ManagedProjectVerificationException(
        'managed revision-3 NPC preparation could not be verified exactly',
      ),
      stackTrace,
    );
  }

  Never _throwRevision3NpcProfileEditSeedError(
    Object error,
    StackTrace stackTrace,
  ) {
    if (error is ArgumentError || error is FormatException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    _requiresReopen = true;
    if (error is ManagedProjectSessionException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    Error.throwWithStackTrace(
      const ManagedProjectVerificationException(
        'managed revision-3 NPC profile seed could not be derived exactly',
      ),
      stackTrace,
    );
  }

  Never _throwRevision3NpcProfileEditPrepareError(
    Object error,
    StackTrace stackTrace,
  ) {
    if (error is ModFfiException) {
      if (error.code == 'AUTHORING_REVISION3_NPC_PROFILE_HEAD_CONFLICT') {
        _requiresReopen = true;
        Error.throwWithStackTrace(
          ManagedProjectHeadConflictException(error.message),
          stackTrace,
        );
      }
      if (_revision3NpcProfileEditPrepareErrorIsRetryable(error.code)) {
        Error.throwWithStackTrace(error, stackTrace);
      }
      _requiresReopen = true;
      Error.throwWithStackTrace(
        ManagedProjectVerificationException(error.message),
        stackTrace,
      );
    }
    if (error is ArgumentError || error is FormatException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    _requiresReopen = true;
    if (error is ManagedProjectSessionException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    Error.throwWithStackTrace(
      const ManagedProjectVerificationException(
        'managed revision-3 NPC profile edit could not be prepared and verified exactly',
      ),
      stackTrace,
    );
  }

  Never _throwRevision3DialogLinePrepareError(
    Object error,
    StackTrace stackTrace,
  ) {
    if (error is ModFfiException) {
      if (error.code == 'AUTHORING_REVISION3_DIALOG_HEAD_CONFLICT') {
        _requiresReopen = true;
        Error.throwWithStackTrace(
          ManagedProjectHeadConflictException(error.message),
          stackTrace,
        );
      }
      if (_revision3DialogLinePrepareErrorIsRetryable(error.code)) {
        Error.throwWithStackTrace(error, stackTrace);
      }
      _requiresReopen = true;
      Error.throwWithStackTrace(
        ManagedProjectVerificationException(error.message),
        stackTrace,
      );
    }
    if (error is ArgumentError || error is FormatException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    _requiresReopen = true;
    if (error is ManagedProjectSessionException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    Error.throwWithStackTrace(
      const ManagedProjectVerificationException(
        'managed revision-3 dialog-line preparation could not be verified exactly',
      ),
      stackTrace,
    );
  }

  Never _throwRevision3DialogLocalizationReadError(
    Object error,
    StackTrace stackTrace,
  ) {
    if (error is ModFfiException) {
      if (error.code ==
          'AUTHORING_REVISION3_DIALOG_LOCALIZATION_HEAD_CONFLICT') {
        _requiresReopen = true;
        Error.throwWithStackTrace(
          ManagedProjectHeadConflictException(error.message),
          stackTrace,
        );
      }
      if (_revision3DialogLocalizationReadErrorIsRetryable(error.code)) {
        Error.throwWithStackTrace(error, stackTrace);
      }
      _requiresReopen = true;
      Error.throwWithStackTrace(
        ManagedProjectVerificationException(error.message),
        stackTrace,
      );
    }
    if (error is ArgumentError || error is FormatException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    _requiresReopen = true;
    if (error is ManagedProjectSessionException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    Error.throwWithStackTrace(
      const ManagedProjectVerificationException(
        'managed revision-3 dialog localization could not be read and verified exactly',
      ),
      stackTrace,
    );
  }

  Never _throwRevision3DialogLocalizationEditReadError(
    Object error,
    StackTrace stackTrace,
  ) {
    if (error is ModFfiException) {
      if (error.code ==
          'AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_HEAD_CONFLICT') {
        _requiresReopen = true;
        Error.throwWithStackTrace(
          ManagedProjectHeadConflictException(error.message),
          stackTrace,
        );
      }
      if (_revision3DialogLocalizationEditReadErrorIsRetryable(error.code)) {
        Error.throwWithStackTrace(error, stackTrace);
      }
      _requiresReopen = true;
      Error.throwWithStackTrace(
        ManagedProjectVerificationException(error.message),
        stackTrace,
      );
    }
    if (error is ArgumentError ||
        error is FormatException ||
        error is ManagedRevision3DialogLocalizationEditStaleException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    _requiresReopen = true;
    if (error is ManagedProjectSessionException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    Error.throwWithStackTrace(
      const ManagedProjectVerificationException(
        'managed revision-3 localization-edit seed could not be read and verified exactly',
      ),
      stackTrace,
    );
  }

  Never _throwRevision3DialogLocalizationEditPrepareError(
    Object error,
    StackTrace stackTrace,
  ) {
    if (error is ModFfiException) {
      if (error.code ==
          'AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_HEAD_CONFLICT') {
        _requiresReopen = true;
        Error.throwWithStackTrace(
          ManagedProjectHeadConflictException(error.message),
          stackTrace,
        );
      }
      if (_revision3DialogLocalizationEditPrepareErrorIsRetryable(error.code)) {
        Error.throwWithStackTrace(error, stackTrace);
      }
      _requiresReopen = true;
      Error.throwWithStackTrace(
        ManagedProjectVerificationException(error.message),
        stackTrace,
      );
    }
    if (error is ArgumentError ||
        error is FormatException ||
        error is ManagedRevision3DialogLocalizationEditStaleException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    _requiresReopen = true;
    if (error is ManagedProjectSessionException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    Error.throwWithStackTrace(
      const ManagedProjectVerificationException(
        'managed revision-3 localization edit could not be prepared and verified exactly',
      ),
      stackTrace,
    );
  }

  Never _throwRevision3VoicePrepareError(Object error, StackTrace stackTrace) {
    if (error is ModFfiException) {
      if (error.code == 'AUTHORING_REVISION3_VOICE_HEAD_CONFLICT') {
        _requiresReopen = true;
        Error.throwWithStackTrace(
          ManagedProjectHeadConflictException(error.message),
          stackTrace,
        );
      }
      if (_revision3VoicePrepareErrorIsRetryable(error.code)) {
        // Input, semantic, capacity, and source-stability failures are safe to retry after the
        // caller's exact fixed-head recheck. Native preparation can leave only immutable CAS
        // orphans and never publishes the fixed head.
        Error.throwWithStackTrace(error, stackTrace);
      }
      // Fail closed for every Store/integrity code and every future unknown native code.
      _requiresReopen = true;
      Error.throwWithStackTrace(
        ManagedProjectVerificationException(error.message),
        stackTrace,
      );
    }
    if (error is ArgumentError || error is FormatException) {
      // These are local request-construction/preflight failures before native work begins.
      Error.throwWithStackTrace(error, stackTrace);
    }
    _requiresReopen = true;
    if (error is ManagedProjectSessionException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    Error.throwWithStackTrace(
      const ManagedProjectVerificationException(
        'managed revision-3 Voice preparation could not be verified exactly',
      ),
      stackTrace,
    );
  }

  Never _throwRevision3VoicePreviewReadError(
    Object error,
    StackTrace stackTrace,
  ) {
    if (error is Revision3VoiceTakePreviewMaterializationCleanupException) {
      // Preserve the bounded cleanup owner, but classify its primary failure
      // exactly as if it had crossed this boundary directly.
      if (_revision3VoicePreviewMaterializationFailureRequiresReopen(
        error.materializationCause,
      )) {
        _requiresReopen = true;
      }
      Error.throwWithStackTrace(error, stackTrace);
    }
    if (error is Revision3VoiceTakePreviewCleanupException) {
      // A cleanup-only failure is capability-local and says nothing about
      // managed Store integrity. Playback still owns its retry lifecycle.
      Error.throwWithStackTrace(error, stackTrace);
    }
    if (error is Revision3VoiceTakePreviewVerificationException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    if (error is ModFfiException) {
      if (_revision3VoicePreviewReadErrorIsRetryable(error.code)) {
        // readExact has independently proved that the fixed head is still the
        // session basis. Exact graph-leaf drift is safe to refresh, while
        // temporary output-capability failures are safe to retry locally.
        Error.throwWithStackTrace(error, stackTrace);
      }
      _requiresReopen = true;
      Error.throwWithStackTrace(
        ManagedProjectVerificationException(error.message),
        stackTrace,
      );
    }
    if (error is ArgumentError ||
        error is FormatException ||
        error is UnsupportedError) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    _requiresReopen = true;
    if (error is ManagedProjectSessionException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    Error.throwWithStackTrace(
      const ManagedProjectVerificationException(
        'managed revision-3 Voice preview could not be read and verified exactly',
      ),
      stackTrace,
    );
  }

  Never _throwRevision3VoiceMediaQaReadError(
    Object error,
    StackTrace stackTrace,
  ) {
    if (error is ModFfiException) {
      if (_revision3VoiceMediaQaReadErrorIsRetryable(error.code)) {
        // readExact independently proved that the fixed head is still exact.
        // A stale line/locale/slot/take/asset selection can be refreshed.
        Error.throwWithStackTrace(error, stackTrace);
      }
      _requiresReopen = true;
      Error.throwWithStackTrace(
        ManagedProjectVerificationException(error.message),
        stackTrace,
      );
    }
    if (error is ArgumentError ||
        error is FormatException ||
        error is UnsupportedError) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    _requiresReopen = true;
    if (error is ManagedProjectSessionException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    Error.throwWithStackTrace(
      const ManagedProjectVerificationException(
        'managed revision-3 Voice media QA could not be read and verified exactly',
      ),
      stackTrace,
    );
  }

  Never _throwRevision3VoiceBatchPlanError(
    Object error,
    StackTrace stackTrace,
  ) {
    if (error is ModFfiException) {
      if (error.code == 'AUTHORING_REVISION3_VOICE_BATCH_HEAD_CONFLICT') {
        _requiresReopen = true;
        Error.throwWithStackTrace(
          ManagedProjectHeadConflictException(error.message),
          stackTrace,
        );
      }
      if (_revision3VoiceBatchPlanErrorIsRetryable(error.code)) {
        Error.throwWithStackTrace(error, stackTrace);
      }
      _requiresReopen = true;
      Error.throwWithStackTrace(
        ManagedProjectVerificationException(error.message),
        stackTrace,
      );
    }
    if (error is ArgumentError || error is UnsupportedError) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    _requiresReopen = true;
    if (error is ManagedProjectSessionException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    Error.throwWithStackTrace(
      const ManagedProjectVerificationException(
        'managed revision-3 Voice folder plan could not be verified exactly',
      ),
      stackTrace,
    );
  }

  Never _throwRevision3VoiceBatchPrepareError(
    Object error,
    StackTrace stackTrace,
  ) {
    if (error is ModFfiException) {
      if (error.code == 'AUTHORING_REVISION3_VOICE_BATCH_HEAD_CONFLICT') {
        _requiresReopen = true;
        Error.throwWithStackTrace(
          ManagedProjectHeadConflictException(error.message),
          stackTrace,
        );
      }
      if (error.code == 'AUTHORING_REVISION3_VOICE_BATCH_PLAN_CHANGED' ||
          error.code == 'AUTHORING_REVISION3_VOICE_BATCH_NOT_READY') {
        Error.throwWithStackTrace(
          const Revision3VoiceBatchStaleCheckpointException(),
          stackTrace,
        );
      }
      if (_revision3VoiceBatchPrepareErrorIsRetryable(error.code)) {
        Error.throwWithStackTrace(error, stackTrace);
      }
      _requiresReopen = true;
      Error.throwWithStackTrace(
        ManagedProjectVerificationException(error.message),
        stackTrace,
      );
    }
    if (error is ArgumentError ||
        error is UnsupportedError ||
        error is Revision3VoiceBatchStaleCheckpointException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    _requiresReopen = true;
    if (error is ManagedProjectSessionException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    Error.throwWithStackTrace(
      const ManagedProjectVerificationException(
        'managed revision-3 Voice folder import could not be prepared and verified exactly',
      ),
      stackTrace,
    );
  }

  Never _throwRevision3VoiceTargetPrepareError(
    Object error,
    StackTrace stackTrace,
  ) {
    if (error is ModFfiException) {
      if (error.code == 'AUTHORING_REVISION3_VOICE_TARGET_HEAD_CONFLICT') {
        _requiresReopen = true;
        Error.throwWithStackTrace(
          ManagedProjectHeadConflictException(error.message),
          stackTrace,
        );
      }
      if (_revision3VoiceTargetPrepareErrorIsRetryable(error.code)) {
        Error.throwWithStackTrace(error, stackTrace);
      }
      _requiresReopen = true;
      Error.throwWithStackTrace(
        ManagedProjectVerificationException(error.message),
        stackTrace,
      );
    }
    if (error is ArgumentError || error is FormatException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    _requiresReopen = true;
    if (error is ManagedProjectSessionException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    Error.throwWithStackTrace(
      const ManagedProjectVerificationException(
        'managed revision-3 Voice target preparation could not be verified exactly',
      ),
      stackTrace,
    );
  }

  Never _throwRevision3VoiceSelectionPrepareError(
    Object error,
    StackTrace stackTrace,
  ) {
    if (error is ModFfiException) {
      if (error.code == 'AUTHORING_REVISION3_VOICE_SELECTION_HEAD_CONFLICT') {
        _requiresReopen = true;
        Error.throwWithStackTrace(
          ManagedProjectHeadConflictException(error.message),
          stackTrace,
        );
      }
      if (_revision3VoiceSelectionPrepareErrorIsRetryable(error.code)) {
        Error.throwWithStackTrace(error, stackTrace);
      }
      _requiresReopen = true;
      Error.throwWithStackTrace(
        ManagedProjectVerificationException(error.message),
        stackTrace,
      );
    }
    if (error is ArgumentError || error is FormatException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    _requiresReopen = true;
    if (error is ManagedProjectSessionException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    Error.throwWithStackTrace(
      const ManagedProjectVerificationException(
        'managed revision-3 Voice selection preparation could not be verified exactly',
      ),
      stackTrace,
    );
  }

  Never _throwRevision3VoiceTakeRemovalPrepareError(
    Object error,
    StackTrace stackTrace,
  ) {
    if (error is ModFfiException) {
      if (error.code ==
          'AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_HEAD_CONFLICT') {
        _requiresReopen = true;
        Error.throwWithStackTrace(
          ManagedProjectHeadConflictException(error.message),
          stackTrace,
        );
      }
      if (_revision3VoiceTakeRemovalPrepareErrorIsRetryable(error.code)) {
        Error.throwWithStackTrace(error, stackTrace);
      }
      _requiresReopen = true;
      Error.throwWithStackTrace(
        ManagedProjectVerificationException(error.message),
        stackTrace,
      );
    }
    if (error is ArgumentError || error is FormatException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    _requiresReopen = true;
    if (error is ManagedProjectSessionException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    Error.throwWithStackTrace(
      const ManagedProjectVerificationException(
        'managed revision-3 Voice take removal could not be prepared and verified exactly',
      ),
      stackTrace,
    );
  }

  Never _throwRevision3DialogVoiceSlotRemovalPrepareError(
    Object error,
    StackTrace stackTrace,
  ) {
    if (error is ModFfiException) {
      if (error.code ==
          'AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_HEAD_CONFLICT') {
        _requiresReopen = true;
        Error.throwWithStackTrace(
          ManagedProjectHeadConflictException(error.message),
          stackTrace,
        );
      }
      if (_revision3DialogVoiceSlotRemovalPrepareErrorIsRetryable(error.code)) {
        Error.throwWithStackTrace(error, stackTrace);
      }
      _requiresReopen = true;
      Error.throwWithStackTrace(
        ManagedProjectVerificationException(error.message),
        stackTrace,
      );
    }
    if (error is ArgumentError || error is FormatException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    _requiresReopen = true;
    if (error is ManagedProjectSessionException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    Error.throwWithStackTrace(
      const ManagedProjectVerificationException(
        'managed revision-3 dialog Voice slot removal could not be prepared and verified exactly',
      ),
      stackTrace,
    );
  }

  Never _throwRevision3VoiceTakeStatusPrepareError(
    Object error,
    StackTrace stackTrace,
  ) {
    if (error is ModFfiException) {
      if (error.code == 'AUTHORING_REVISION3_VOICE_TAKE_STATUS_HEAD_CONFLICT') {
        _requiresReopen = true;
        Error.throwWithStackTrace(
          ManagedProjectHeadConflictException(error.message),
          stackTrace,
        );
      }
      if (_revision3VoiceTakeStatusPrepareErrorIsRetryable(error.code)) {
        Error.throwWithStackTrace(error, stackTrace);
      }
      _requiresReopen = true;
      Error.throwWithStackTrace(
        ManagedProjectVerificationException(error.message),
        stackTrace,
      );
    }
    if (error is ArgumentError || error is FormatException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    _requiresReopen = true;
    if (error is ManagedProjectSessionException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    Error.throwWithStackTrace(
      const ManagedProjectVerificationException(
        'managed revision-3 Voice take status preparation could not be verified exactly',
      ),
      stackTrace,
    );
  }

  Never _throwRevision3VoiceBuildPlanError(
    Object error,
    StackTrace stackTrace,
  ) {
    if (error is ModFfiException) {
      if (error.code == 'AUTHORING_REVISION3_VOICE_PLAN_HEAD_CONFLICT') {
        _requiresReopen = true;
        Error.throwWithStackTrace(
          ManagedProjectHeadConflictException(error.message),
          stackTrace,
        );
      }
      if (_revision3VoiceBuildPlanErrorIsRetryable(error.code)) {
        Error.throwWithStackTrace(error, stackTrace);
      }
      _requiresReopen = true;
      Error.throwWithStackTrace(
        ManagedProjectVerificationException(error.message),
        stackTrace,
      );
    }
    _requiresReopen = true;
    if (error is ManagedProjectSessionException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    Error.throwWithStackTrace(
      const ManagedProjectVerificationException(
        'managed revision-3 Voice build plan could not be verified exactly',
      ),
      stackTrace,
    );
  }

  Never _throwRevision3VoiceBuildError(Object error, StackTrace stackTrace) {
    if (error is ModFfiException) {
      if (error.code == 'AUTHORING_REVISION3_VOICE_BUILD_HEAD_CONFLICT') {
        _requiresReopen = true;
        Error.throwWithStackTrace(
          ManagedProjectHeadConflictException(error.message),
          stackTrace,
        );
      }
      if (error.code ==
          'AUTHORING_REVISION3_VOICE_BUILD_PUBLICATION_UNCONFIRMED') {
        // The Store and fixed head are still exact, but the atomic output
        // publication may already have succeeded. Preserve the native code so
        // the build surface can stop retries and tell the author to inspect
        // that exact output instead of poisoning an otherwise intact project.
        Error.throwWithStackTrace(error, stackTrace);
      }
      if (_revision3VoiceBuildErrorIsRetryable(error.code)) {
        Error.throwWithStackTrace(error, stackTrace);
      }
      _requiresReopen = true;
      Error.throwWithStackTrace(
        ManagedProjectVerificationException(error.message),
        stackTrace,
      );
    }
    if (error is ArgumentError || error is FormatException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    _requiresReopen = true;
    if (error is ManagedProjectSessionException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    Error.throwWithStackTrace(
      const ManagedProjectVerificationException(
        'managed revision-3 Voice build could not be verified exactly',
      ),
      stackTrace,
    );
  }

  Never _throwRevision3ExactSnapshotExportError(
    Object error,
    StackTrace stackTrace,
  ) {
    if (error is ModFfiException) {
      if (error.code == 'AUTHORING_REVISION3_EXPORT_HEAD_CONFLICT') {
        _requiresReopen = true;
        Error.throwWithStackTrace(
          ManagedProjectHeadConflictException(error.message),
          stackTrace,
        );
      }
      if (_revision3ExactSnapshotExportErrorIsRetryable(error.code)) {
        Error.throwWithStackTrace(error, stackTrace);
      }
      _requiresReopen = true;
      if (_revision3ExactSnapshotExportErrorIsKnownPrepublication(error.code)) {
        Error.throwWithStackTrace(
          ManagedRevision3ExactSnapshotExportPrepublicationException(
            code: error.code,
            message: error.message,
          ),
          stackTrace,
        );
      }
      Error.throwWithStackTrace(
        ManagedProjectVerificationException(error.message),
        stackTrace,
      );
    }
    // ArgumentError is produced by local request preflight before the Store
    // capability is invoked. A raw FormatException is not granted that
    // authority: an alternate capability could throw it after publication.
    if (error is ArgumentError) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    _requiresReopen = true;
    if (error is ManagedProjectSessionException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    Error.throwWithStackTrace(
      const ManagedProjectVerificationException(
        'managed revision-3 exact snapshot export could not be verified exactly',
      ),
      stackTrace,
    );
  }

  Never _throwRevision3ReviewedDataAssetBuildError(
    Object error,
    StackTrace stackTrace,
  ) {
    if (error is ModFfiException) {
      if (error.code == 'AUTHORING_REVISION3_DATAASSET_BUILD_HEAD_CONFLICT') {
        _requiresReopen = true;
        Error.throwWithStackTrace(
          ManagedProjectHeadConflictException(error.message),
          stackTrace,
        );
      }
      if (_revision3ReviewedDataAssetBuildErrorIsRetryable(error.code)) {
        Error.throwWithStackTrace(error, stackTrace);
      }
      _requiresReopen = true;
      Error.throwWithStackTrace(
        ManagedProjectVerificationException(error.message),
        stackTrace,
      );
    }
    if (error is ArgumentError || error is FormatException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    _requiresReopen = true;
    if (error is ManagedProjectSessionException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    Error.throwWithStackTrace(
      const ManagedProjectVerificationException(
        'managed reviewed DataAsset build could not be verified exactly',
      ),
      stackTrace,
    );
  }

  Never _throwRevision3StoryDraftRemovalError(
    Object error,
    StackTrace stackTrace,
  ) {
    if (error is ModFfiException) {
      if (error.code ==
          'AUTHORING_REVISION3_STORY_DRAFT_REMOVE_HEAD_CONFLICT') {
        _requiresReopen = true;
        Error.throwWithStackTrace(
          ManagedProjectHeadConflictException(error.message),
          stackTrace,
        );
      }
      if (_revision3StoryDraftRemovalErrorIsRetryable(error.code)) {
        // These closed semantic/capacity conflicts happen before fixed-head
        // publication. The exact disk head was rechecked by the caller.
        Error.throwWithStackTrace(error, stackTrace);
      }
      // Malformed responses, Store/invariant failures, and every future
      // unknown code are uncertain and permanently remove authoring authority
      // until verified recovery/reopen.
      _requiresReopen = true;
      Error.throwWithStackTrace(
        ManagedProjectVerificationException(error.message),
        stackTrace,
      );
    }
    if (error is ArgumentError) {
      // Local path/envelope preflight completes before native preparation.
      Error.throwWithStackTrace(error, stackTrace);
    }
    _requiresReopen = true;
    if (error is ManagedProjectSessionException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    Error.throwWithStackTrace(
      const ManagedProjectVerificationException(
        'managed revision-3 Story Draft removal could not be prepared and verified exactly',
      ),
      stackTrace,
    );
  }

  Never _throwRevision3DataAssetError(Object error, StackTrace stackTrace) {
    if (error is ModFfiException) {
      if (error.code == 'AUTHORING_REVISION3_DATAASSET_HEAD_CONFLICT') {
        _requiresReopen = true;
        Error.throwWithStackTrace(
          ManagedProjectHeadConflictException(error.message),
          stackTrace,
        );
      }
      if (_revision3DataAssetErrorIsRetryable(error.code)) {
        // Bounded input, semantic, capacity, live-generation, and target
        // conflicts occur before fixed-head publication. After the exact disk
        // head recheck above, the caller may correct the input and retry.
        Error.throwWithStackTrace(error, stackTrace);
      }
      // Fail closed for every Store/integrity code and every future unknown
      // native code. New failures must be deliberately classified before a
      // poisoned session may retry them.
      _requiresReopen = true;
      Error.throwWithStackTrace(
        ManagedProjectVerificationException(error.message),
        stackTrace,
      );
    }
    if (error is ArgumentError) {
      // ModFfi's allocation-free path/envelope preflight is entirely local and occurs before the
      // native command. The exact disk head was rechecked, so the caller may fix the input and
      // retry without reopening the project.
      Error.throwWithStackTrace(error, stackTrace);
    }
    _requiresReopen = true;
    if (error is ManagedProjectSessionException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    Error.throwWithStackTrace(
      const ManagedProjectVerificationException(
        'managed revision-3 DataAsset operation could not be verified exactly',
      ),
      stackTrace,
    );
  }

  Never _throwRevision3InstalledDataAssetEditError(
    Object error,
    StackTrace stackTrace,
  ) {
    if (error is ModFfiException) {
      if (const {
        'AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_HEAD_CONFLICT',
        'AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_HEAD_CONFLICT',
        'AUTHORING_REVISION3_DATAASSET_HEAD_CONFLICT',
      }.contains(error.code)) {
        _requiresReopen = true;
        Error.throwWithStackTrace(
          ManagedProjectHeadConflictException(error.message),
          stackTrace,
        );
      }
      if (_revision3InstalledDataAssetEditErrorIsRetryable(error.code) ||
          _revision3InstalledDataAssetInspectionErrorIsRetryable(error.code)) {
        // Source, selector, inventory, capacity, and bounded response failures
        // occur before fixed-head publication. Immutable CAS orphans are safe;
        // the caller can obtain a fresh installed inspection and retry.
        Error.throwWithStackTrace(error, stackTrace);
      }
      if (error.code.startsWith('AUTHORING_REVISION3_DATAASSET_')) {
        _throwRevision3DataAssetError(error, stackTrace);
      }
      _requiresReopen = true;
      Error.throwWithStackTrace(
        ManagedProjectVerificationException(error.message),
        stackTrace,
      );
    }
    if (error is ArgumentError || error is FormatException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    _requiresReopen = true;
    Error.throwWithStackTrace(
      const ManagedProjectVerificationException(
        'managed revision-3 installed DataAsset edit could not be verified exactly',
      ),
      stackTrace,
    );
  }

  Never _throwRevision3ContentReadError(Object error, StackTrace stackTrace) {
    if (error is ModFfiException) {
      if (error.code == 'AUTHORING_REVISION3_CONTENT_HEAD_CONFLICT') {
        _requiresReopen = true;
        Error.throwWithStackTrace(
          ManagedProjectHeadConflictException(error.message),
          stackTrace,
        );
      }
      if (_revision3ContentReadErrorRequiresReopen(error.code)) {
        _requiresReopen = true;
        Error.throwWithStackTrace(
          ManagedProjectVerificationException(error.message),
          stackTrace,
        );
      }
      Error.throwWithStackTrace(error, stackTrace);
    }
    _requiresReopen = true;
    if (error is ManagedProjectSessionException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    Error.throwWithStackTrace(
      const ManagedProjectVerificationException(
        'managed revision-3 content could not be read and verified exactly',
      ),
      stackTrace,
    );
  }

  Never _throwRevision3HistoryReadError(Object error, StackTrace stackTrace) {
    if (error is _ManagedRevision3HistoryFailureWithVerifiedCurrent) {
      Error.throwWithStackTrace(error.error, error.stackTrace);
    }
    if (error is ModFfiException) {
      if (error.code == 'AUTHORING_REVISION3_HISTORY_HEAD_CONFLICT') {
        _requiresReopen = true;
        Error.throwWithStackTrace(
          ManagedProjectHeadConflictException(error.message),
          stackTrace,
        );
      }
      if (_revision3HistoryErrorIsRetryable(error.code)) {
        Error.throwWithStackTrace(error, stackTrace);
      }
      _requiresReopen = true;
      Error.throwWithStackTrace(
        ManagedProjectVerificationException(error.message),
        stackTrace,
      );
    }
    if (error is ArgumentError ||
        error is FormatException ||
        error is UnsupportedError) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    _requiresReopen = true;
    Error.throwWithStackTrace(
      const ManagedProjectVerificationException(
        'managed revision-3 history could not be read and verified exactly',
      ),
      stackTrace,
    );
  }

  Never _throwRevision3HistoryRestorePrepareError(
    Object error,
    StackTrace stackTrace,
  ) {
    if (error is _ManagedRevision3HistoryFailureWithVerifiedCurrent) {
      Error.throwWithStackTrace(error.error, error.stackTrace);
    }
    if (error is ModFfiException) {
      if (error.code == 'AUTHORING_REVISION3_HISTORY_HEAD_CONFLICT') {
        _requiresReopen = true;
        Error.throwWithStackTrace(
          ManagedProjectHeadConflictException(error.message),
          stackTrace,
        );
      }
      if (_revision3HistoryErrorIsRetryable(error.code)) {
        // Reachability, lineage, retention, and revision-overflow failures
        // happen before fixed-head publication. They can leave only immutable
        // CAS orphans and are safe to resolve by refreshing the timeline.
        Error.throwWithStackTrace(error, stackTrace);
      }
      _requiresReopen = true;
      Error.throwWithStackTrace(
        ManagedProjectVerificationException(error.message),
        stackTrace,
      );
    }
    if (error is ArgumentError ||
        error is FormatException ||
        error is UnsupportedError) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    _requiresReopen = true;
    if (error is ManagedProjectSessionException) {
      Error.throwWithStackTrace(error, stackTrace);
    }
    Error.throwWithStackTrace(
      const ManagedProjectVerificationException(
        'managed revision-3 history restore could not be verified exactly',
      ),
      stackTrace,
    );
  }

  Future<void> _saveCapturedInQueue(String capturedProjectJson) async {
    _requireWritableState();
    final oldHead = _opened.head;
    final operations = _ManagedSessionOperations(
      root: root,
      store: _store,
      replacement: _replacement,
    );
    await _requireExactPublishedHead(operations, oldHead);
    final AuthoringWorkingHead preparedHead;
    try {
      preparedHead = await _store.prepareCheckpoint(
        root: root.path,
        expectedHead: oldHead,
        projectJson: capturedProjectJson,
      );
    } on ModFfiException catch (error) {
      if (error.code == 'AUTHORING_STORE_HEAD_CONFLICT') {
        _requiresReopen = true;
        throw ManagedProjectHeadConflictException(error.message);
      }
      if (_prepareErrorRequiresReopen(error.code)) {
        _requiresReopen = true;
        throw ManagedProjectVerificationException(error.message);
      }
      rethrow;
    } on ManagedProjectHeadConflictException {
      _requiresReopen = true;
      rethrow;
    }
    await operations.verifyPreparedCheckpoint(
      preparedHead,
      expectedProjectJson: capturedProjectJson,
    );

    try {
      await operations.publish(preparedHead, expectedHead: oldHead);
    } on AtomicSwapConflictException catch (error) {
      _requiresReopen = true;
      throw ManagedProjectHeadConflictException(error.message);
    } on AtomicSwapException {
      _requiresReopen = true;
      rethrow;
    } catch (_) {
      // Publication has entered the crash-recoverable replacement lane. Even an exception that
      // is not normalized by AtomicByteReplacement (for example a raw filesystem failure from a
      // journal write, rename, delete, or phase hook) can leave the fixed head and its repair
      // journal between generations. Do not permit another edit until open() repairs and fully
      // verifies the authoritative generation.
      _requiresReopen = true;
      rethrow;
    }

    try {
      _opened = await operations.openPublished(
        expectedHead: preparedHead,
        expectedProjectJson: capturedProjectJson,
      );
    } catch (_) {
      _requiresReopen = true;
      rethrow;
    }
  }

  /// Wait for earlier saves, release the OS lock once, and reject new saves.
  Future<void> close() {
    if (_isActiveDeriveCallbackZone) {
      return _reentrantOperation<void>('close');
    }
    final existing = _closeFuture;
    if (existing != null) return existing;
    _closeRequested = true;
    final result = _enqueue(() async {
      try {
        await _lock.release();
      } finally {
        _closed = true;
      }
    }, permitClosing: true);
    _closeFuture = result;
    return result;
  }

  Future<void> _requireExactPublishedHead(
    _ManagedSessionOperations operations,
    AuthoringWorkingHead expectedHead,
  ) async {
    try {
      await operations.requirePublishedHead(expectedHead);
    } on ManagedProjectSessionException {
      _requiresReopen = true;
      rethrow;
    } on FileSystemException {
      _requiresReopen = true;
      throw const ManagedProjectVerificationException(
        'managed project head could not be verified exactly',
      );
    }
  }

  Future<T> _enqueue<T>(
    Future<T> Function() operation, {
    bool permitClosing = false,
  }) {
    if (_closed || (_closeRequested && !permitClosing)) {
      return Future<T>.error(
        const ManagedProjectSessionClosedException(
          'managed project session is closing or closed',
        ),
      );
    }
    final result = _tail.then((_) => operation());
    _tail = result.then<void>((_) {}, onError: (Object _, StackTrace _) {});
    return result;
  }
}

final class _ManagedProjectDeriveZoneToken {
  bool active = true;
}

class _ManagedSessionOperations {
  const _ManagedSessionOperations({
    required this.root,
    required this.store,
    required this.replacement,
  });

  final Directory root;
  final _ManagedCheckpointStore store;
  final AtomicByteReplacement replacement;

  File get headFile => File(p.join(root.path, 'gore-project.json'));

  Future<AtomicRepairOutcome> repairHead() =>
      replacement.repair(target: headFile, validate: _validateHeadCandidate);

  Future<void> verifyPreparedCheckpoint(
    AuthoringWorkingHead head, {
    required String expectedProjectJson,
  }) async {
    final opened = await store.openHeadBytes(
      root: root.path,
      head: head,
      verification: AuthoringAssetVerification.full,
    );
    _requireExactOpened(
      opened,
      expectedHead: head,
      expectedProjectJson: expectedProjectJson,
      context: 'prepared checkpoint',
    );
  }

  Future<void> requirePublishedHead(AuthoringWorkingHead expectedHead) async {
    final actualHead = await _readCanonicalHead(headFile);
    if (actualHead.canonicalJson != expectedHead.canonicalJson) {
      throw const ManagedProjectHeadConflictException(
        'managed project head changed since the session opened it',
      );
    }
  }

  Future<void> publish(
    AuthoringWorkingHead head, {
    required AuthoringWorkingHead? expectedHead,
  }) => replacement.replaceIfUnchanged(
    target: headFile,
    bytes: utf8.encode(head.canonicalJson),
    expectedBytes: expectedHead == null
        ? null
        : utf8.encode(expectedHead.canonicalJson),
    validate: _validateHeadCandidate,
  );

  Future<_ManagedOpenedCheckpoint> openPublished({
    AuthoringWorkingHead? expectedHead,
    String? expectedProjectJson,
  }) async {
    final exactDiskHead = await _readCanonicalHead(headFile);
    final opened = await store.open(
      root: root.path,
      verification: AuthoringAssetVerification.full,
    );
    _requireExactOpened(
      opened,
      expectedHead: expectedHead ?? exactDiskHead,
      expectedProjectJson: expectedProjectJson,
      context: 'published checkpoint',
    );
    if (opened.head.canonicalJson != exactDiskHead.canonicalJson) {
      throw const ManagedProjectVerificationException(
        'native open did not return the exact published head bytes',
      );
    }
    return opened;
  }

  Future<bool> _validateHeadCandidate(File candidate) async {
    try {
      final head = await _readCanonicalHead(candidate);
      final opened = await store.openHeadBytes(
        root: root.path,
        head: head,
        verification: AuthoringAssetVerification.full,
      );
      _requireExactOpened(
        opened,
        expectedHead: head,
        context: 'head candidate',
      );
      return true;
    } catch (_) {
      return false;
    }
  }
}

bool _prepareErrorRequiresReopen(String code) => const {
  'AUTHORING_STORE_HEAD_INVALID',
  'AUTHORING_STORE_HEAD_NONCANONICAL',
  'AUTHORING_STORE_HEAD_LIMIT',
  'AUTHORING_STORE_HEAD_MISSING',
  'AUTHORING_STORE_JSON_INVALID',
  'AUTHORING_STORE_JSON_NONCANONICAL',
  'AUTHORING_STORE_PATH_UNSAFE',
  'AUTHORING_STORE_ROOT_MISSING',
}.contains(code);

bool _sameDraftContentSeal(
  AuthoringDraftContentSeal left,
  AuthoringDraftContentSeal right,
) => left.byteLength == right.byteLength && left.sha256 == right.sha256;

Map<String, String> _revision3DialogLocalizationTexts(
  String projectJson, {
  required String localizationId,
  required int expectedLocalizationRevision,
  required String expectedLocId,
}) {
  try {
    final project = (jsonDecode(projectJson) as Map).cast<String, Object?>();
    final entities = (project['entities'] as Map).cast<String, Object?>();
    final rawEntity = entities[localizationId];
    if (rawEntity is! Map) {
      throw const ManagedRevision3DialogLocalizationEditStaleException();
    }
    final entity = rawEntity.cast<String, Object?>();
    final origin = (entity['origin'] as Map).cast<String, Object?>();
    final payload = (entity['payload'] as Map).cast<String, Object?>();
    final data = (payload['data'] as Map).cast<String, Object?>();
    if (entity['id'] != localizationId ||
        entity['revision'] != expectedLocalizationRevision ||
        origin['type'] != 'new' ||
        payload['kind'] != 'localization_entry' ||
        data['loc_id'] != expectedLocId) {
      throw const ManagedRevision3DialogLocalizationEditStaleException();
    }
    final rawTexts = (data['texts'] as Map).cast<String, Object?>();
    return Map<String, String>.unmodifiable(
      rawTexts.map((locale, text) {
        if (text is! String) {
          throw const ManagedProjectVerificationException(
            'revision-3 localization text is not a string',
          );
        }
        return MapEntry(locale, text);
      }),
    );
  } on ManagedProjectSessionException {
    rethrow;
  } catch (_) {
    throw const ManagedProjectVerificationException(
      'revision-3 localization edit could not bind its exact project entity',
    );
  }
}

bool _sameStrings(List<String> left, List<String> right) {
  if (left.length != right.length) return false;
  for (var index = 0; index < left.length; index++) {
    if (left[index] != right[index]) return false;
  }
  return true;
}

bool _revision3QuestPrepareErrorIsRetryable(String code) => const {
  'AUTHORING_REVISION3_QUEST_ARTIFACT_FAILED',
  'AUTHORING_REVISION3_QUEST_CAPABILITY_FAILED',
  'AUTHORING_REVISION3_QUEST_COLLISION_LIMIT',
  'AUTHORING_REVISION3_QUEST_INPUT_CHANGED',
  'AUTHORING_REVISION3_QUEST_INPUT_LIMIT',
  'AUTHORING_REVISION3_QUEST_INPUT_MISSING',
  'AUTHORING_REVISION3_QUEST_INPUT_UNAVAILABLE',
  'AUTHORING_REVISION3_QUEST_INPUT_UNSAFE',
  'AUTHORING_REVISION3_QUEST_INVENTORY_FAILED',
  'AUTHORING_REVISION3_QUEST_PRISTINE_UNAVAILABLE',
  'AUTHORING_REVISION3_QUEST_PROJECT_LIMIT',
  'AUTHORING_REVISION3_QUEST_PROJECT_TARGET_MISMATCH',
  'AUTHORING_REVISION3_QUEST_RECOVERY_REQUIRED',
  'AUTHORING_REVISION3_QUEST_REJECTED',
  'AUTHORING_REVISION3_QUEST_REQUEST_INVALID',
  'AUTHORING_REVISION3_QUEST_REQUEST_LIMIT',
  'AUTHORING_REVISION3_QUEST_RESPONSE_LIMIT',
  'AUTHORING_REVISION3_QUEST_STORE_GAME_ALIAS',
  'AUTHORING_REVISION3_QUEST_UNSUPPORTED_GENERATION',
}.contains(code);

bool _revision3QuestOutlinePrepareErrorIsRetryable(String code) => const {
  'AUTHORING_REVISION3_QUEST_OUTLINE_INPUT_LIMIT',
  'AUTHORING_REVISION3_QUEST_OUTLINE_NO_CHANGES',
  'AUTHORING_REVISION3_QUEST_OUTLINE_PROJECT_CONFLICT',
  'AUTHORING_REVISION3_QUEST_OUTLINE_PROJECT_LIMIT',
  'AUTHORING_REVISION3_QUEST_OUTLINE_QUEST_CONFLICT',
  'AUTHORING_REVISION3_QUEST_OUTLINE_REQUEST_INVALID',
  'AUTHORING_REVISION3_QUEST_OUTLINE_REQUEST_LIMIT',
  'AUTHORING_REVISION3_QUEST_OUTLINE_REQUEST_REJECTED',
  'AUTHORING_REVISION3_QUEST_OUTLINE_REVISION_LIMIT',
  'AUTHORING_REVISION3_QUEST_OUTLINE_SHAPE_CONFLICT',
  'AUTHORING_REVISION3_QUEST_OUTLINE_SIGNED_WIRE_LIMIT',
  'AUTHORING_REVISION3_QUEST_OUTLINE_TARGET_CONFLICT',
  'AUTHORING_REVISION3_QUEST_OUTLINE_V2_INPUT_LIMIT',
  'AUTHORING_REVISION3_QUEST_OUTLINE_V2_MODULE_CONFLICT',
  'AUTHORING_REVISION3_QUEST_OUTLINE_V2_NO_CHANGES',
  'AUTHORING_REVISION3_QUEST_OUTLINE_V2_PLAN_CONFLICT',
  'AUTHORING_REVISION3_QUEST_OUTLINE_V2_PROJECT_CONFLICT',
  'AUTHORING_REVISION3_QUEST_OUTLINE_V2_PROJECT_LIMIT',
  'AUTHORING_REVISION3_QUEST_OUTLINE_V2_QUEST_CONFLICT',
  'AUTHORING_REVISION3_QUEST_OUTLINE_V2_REQUEST_INVALID',
  'AUTHORING_REVISION3_QUEST_OUTLINE_V2_REQUEST_LIMIT',
  'AUTHORING_REVISION3_QUEST_OUTLINE_V2_REQUEST_REJECTED',
  'AUTHORING_REVISION3_QUEST_OUTLINE_V2_REQUIRES_SEMANTIC_QUEST',
  'AUTHORING_REVISION3_QUEST_OUTLINE_V2_RESPONSE_LIMIT',
  'AUTHORING_REVISION3_QUEST_OUTLINE_V2_REVISION_LIMIT',
  'AUTHORING_REVISION3_QUEST_OUTLINE_V2_SIGNED_WIRE_LIMIT',
  'AUTHORING_REVISION3_QUEST_OUTLINE_V2_SLOT_CONFLICT',
  'AUTHORING_REVISION3_QUEST_OUTLINE_V2_STORE_LIMIT',
  'AUTHORING_REVISION3_QUEST_OUTLINE_V2_TARGET_CONFLICT',
}.contains(code);

bool _revision3QuestTransitionsPrepareErrorIsRetryable(String code) => const {
  'AUTHORING_REVISION3_QUEST_TRANSITIONS_INPUT_LIMIT',
  'AUTHORING_REVISION3_QUEST_TRANSITIONS_NO_CHANGES',
  'AUTHORING_REVISION3_QUEST_TRANSITIONS_PROJECT_CONFLICT',
  'AUTHORING_REVISION3_QUEST_TRANSITIONS_PROJECT_LIMIT',
  'AUTHORING_REVISION3_QUEST_TRANSITIONS_QUEST_CONFLICT',
  'AUTHORING_REVISION3_QUEST_TRANSITIONS_REQUEST_INVALID',
  'AUTHORING_REVISION3_QUEST_TRANSITIONS_REQUEST_LIMIT',
  'AUTHORING_REVISION3_QUEST_TRANSITIONS_REQUEST_REJECTED',
  'AUTHORING_REVISION3_QUEST_TRANSITIONS_REVISION_LIMIT',
  'AUTHORING_REVISION3_QUEST_TRANSITIONS_SIGNED_WIRE_LIMIT',
  'AUTHORING_REVISION3_QUEST_TRANSITIONS_STORE_LIMIT',
  'AUTHORING_REVISION3_QUEST_TRANSITIONS_TARGET_CONFLICT',
  'AUTHORING_REVISION3_QUEST_TRANSITIONS_TRANSITION_PLAN_CONFLICT',
}.contains(code);

bool _revision3QuestTranscriptPrepareErrorIsRetryable(String code) => const {
  'AUTHORING_REVISION3_QUEST_TRANSCRIPT_BINDING_CONFLICT',
  'AUTHORING_REVISION3_QUEST_TRANSCRIPT_DIALOG_CONFLICT',
  'AUTHORING_REVISION3_QUEST_TRANSCRIPT_INDEX_CONFLICT',
  'AUTHORING_REVISION3_QUEST_TRANSCRIPT_INPUT_LIMIT',
  'AUTHORING_REVISION3_QUEST_TRANSCRIPT_NO_CHANGES',
  'AUTHORING_REVISION3_QUEST_TRANSCRIPT_PROJECT_CONFLICT',
  'AUTHORING_REVISION3_QUEST_TRANSCRIPT_PROJECT_LIMIT',
  'AUTHORING_REVISION3_QUEST_TRANSCRIPT_QUEST_CONFLICT',
  'AUTHORING_REVISION3_QUEST_TRANSCRIPT_REQUEST_INVALID',
  'AUTHORING_REVISION3_QUEST_TRANSCRIPT_REQUEST_LIMIT',
  'AUTHORING_REVISION3_QUEST_TRANSCRIPT_RESPONSE_LIMIT',
  'AUTHORING_REVISION3_QUEST_TRANSCRIPT_REVISION_LIMIT',
  'AUTHORING_REVISION3_QUEST_TRANSCRIPT_SIGNED_WIRE_LIMIT',
  'AUTHORING_REVISION3_QUEST_TRANSCRIPT_STORE_LIMIT',
  'AUTHORING_REVISION3_QUEST_TRANSCRIPT_TARGET_CONFLICT',
}.contains(code);

bool _revision3NpcGreetingPrepareErrorIsRetryable(String code) => const {
  'AUTHORING_REVISION3_NPC_GREETING_BINDING_CONFLICT',
  'AUTHORING_REVISION3_NPC_GREETING_DIALOG_CONFLICT',
  'AUTHORING_REVISION3_NPC_GREETING_INDEX_CONFLICT',
  'AUTHORING_REVISION3_NPC_GREETING_INPUT_LIMIT',
  'AUTHORING_REVISION3_NPC_GREETING_NO_CHANGES',
  'AUTHORING_REVISION3_NPC_GREETING_NPC_CONFLICT',
  'AUTHORING_REVISION3_NPC_GREETING_PROJECT_CONFLICT',
  'AUTHORING_REVISION3_NPC_GREETING_PROJECT_INVALID',
  'AUTHORING_REVISION3_NPC_GREETING_PROJECT_LIMIT',
  'AUTHORING_REVISION3_NPC_GREETING_REQUEST_INVALID',
  'AUTHORING_REVISION3_NPC_GREETING_REQUEST_LIMIT',
  'AUTHORING_REVISION3_NPC_GREETING_REQUEST_REJECTED',
  'AUTHORING_REVISION3_NPC_GREETING_RESPONSE_LIMIT',
  'AUTHORING_REVISION3_NPC_GREETING_REVISION_LIMIT',
  'AUTHORING_REVISION3_NPC_GREETING_SIGNED_WIRE_LIMIT',
  'AUTHORING_REVISION3_NPC_GREETING_STORE_LIMIT',
  'AUTHORING_REVISION3_NPC_GREETING_TARGET_CONFLICT',
}.contains(code);

bool _revision3QuestContextPrepareErrorIsRetryable(String code) => const {
  'AUTHORING_REVISION3_QUEST_CONTEXT_ARTIFACT_FAILED',
  'AUTHORING_REVISION3_QUEST_CONTEXT_CAPABILITY_FAILED',
  'AUTHORING_REVISION3_QUEST_CONTEXT_CATALOG_CONFLICT',
  'AUTHORING_REVISION3_QUEST_CONTEXT_COLLISION_LIMIT',
  'AUTHORING_REVISION3_QUEST_CONTEXT_INPUT_CHANGED',
  'AUTHORING_REVISION3_QUEST_CONTEXT_INPUT_LIMIT',
  'AUTHORING_REVISION3_QUEST_CONTEXT_INPUT_MISSING',
  'AUTHORING_REVISION3_QUEST_CONTEXT_INPUT_UNAVAILABLE',
  'AUTHORING_REVISION3_QUEST_CONTEXT_INPUT_UNSAFE',
  'AUTHORING_REVISION3_QUEST_CONTEXT_INVENTORY_FAILED',
  'AUTHORING_REVISION3_QUEST_CONTEXT_NO_CHANGES',
  'AUTHORING_REVISION3_QUEST_CONTEXT_PRISTINE_UNAVAILABLE',
  'AUTHORING_REVISION3_QUEST_CONTEXT_PROJECT_CONFLICT',
  'AUTHORING_REVISION3_QUEST_CONTEXT_PROJECT_INVALID',
  'AUTHORING_REVISION3_QUEST_CONTEXT_PROJECT_LIMIT',
  'AUTHORING_REVISION3_QUEST_CONTEXT_QUEST_CONFLICT',
  'AUTHORING_REVISION3_QUEST_CONTEXT_RECOVERY_REQUIRED',
  'AUTHORING_REVISION3_QUEST_CONTEXT_REQUEST_INVALID',
  'AUTHORING_REVISION3_QUEST_CONTEXT_REQUEST_LIMIT',
  'AUTHORING_REVISION3_QUEST_CONTEXT_REQUEST_REJECTED',
  'AUTHORING_REVISION3_QUEST_CONTEXT_RESPONSE_LIMIT',
  'AUTHORING_REVISION3_QUEST_CONTEXT_REVISION_LIMIT',
  'AUTHORING_REVISION3_QUEST_CONTEXT_SIGNED_WIRE_LIMIT',
  'AUTHORING_REVISION3_QUEST_CONTEXT_STORE_GAME_ALIAS',
  'AUTHORING_REVISION3_QUEST_CONTEXT_STORE_LIMIT',
  'AUTHORING_REVISION3_QUEST_CONTEXT_TARGET_CONFLICT',
  'AUTHORING_REVISION3_QUEST_CONTEXT_UNSUPPORTED_GENERATION',
}.contains(code);

bool _revision3QuestSourceInspectionErrorIsRetryable(String code) => const {
  'AUTHORING_REVISION3_QUEST_INSPECTION_COLLISION_LIMIT',
  'AUTHORING_REVISION3_QUEST_INSPECTION_FAILED',
  'AUTHORING_REVISION3_QUEST_INSPECTION_INPUT_CHANGED',
  'AUTHORING_REVISION3_QUEST_INSPECTION_INPUT_LIMIT',
  'AUTHORING_REVISION3_QUEST_INSPECTION_INPUT_MISSING',
  'AUTHORING_REVISION3_QUEST_INSPECTION_INPUT_UNAVAILABLE',
  'AUTHORING_REVISION3_QUEST_INSPECTION_INPUT_UNSAFE',
  'AUTHORING_REVISION3_QUEST_INSPECTION_INVENTORY_FAILED',
  'AUTHORING_REVISION3_QUEST_INSPECTION_PROJECT_INVALID',
  'AUTHORING_REVISION3_QUEST_INSPECTION_PROJECT_TARGET_MISMATCH',
  'AUTHORING_REVISION3_QUEST_INSPECTION_QUEST_INVALID',
  'AUTHORING_REVISION3_QUEST_INSPECTION_RECOVERY_REQUIRED',
  'AUTHORING_REVISION3_QUEST_INSPECTION_REQUEST_INVALID',
  'AUTHORING_REVISION3_QUEST_INSPECTION_RESPONSE_LIMIT',
  'AUTHORING_REVISION3_QUEST_INSPECTION_UNSUPPORTED_GENERATION',
}.contains(code);

bool _revision3NpcSourceInspectionErrorIsRetryable(String code) => const {
  'AUTHORING_REVISION3_NPC_INSPECTION_FAILED',
  'AUTHORING_REVISION3_NPC_INSPECTION_INPUT_LIMIT',
  'AUTHORING_REVISION3_NPC_INSPECTION_NPC_INVALID',
  'AUTHORING_REVISION3_NPC_INSPECTION_PROJECT_INVALID',
  'AUTHORING_REVISION3_NPC_INSPECTION_REQUEST_INVALID',
  'AUTHORING_REVISION3_NPC_INSPECTION_RESPONSE_LIMIT',
}.contains(code);

bool _revision3ManagedCompilerCheckErrorIsRetryable(String code) => const {
  'AUTHORING_REVISION3_COMPILER_REQUEST_INVALID',
  'AUTHORING_REVISION3_COMPILER_INPUT_LIMIT',
  'AUTHORING_REVISION3_COMPILER_HEAD_INVALID',
  'AUTHORING_REVISION3_COMPILER_STORE_ROOT_MISSING',
  'AUTHORING_REVISION3_COMPILER_HEAD_MISSING',
  'AUTHORING_REVISION3_COMPILER_ENTITY_INVALID',
}.contains(code);

bool _revision3DataAssetPackageIndexErrorIsRetryable(String code) => const {
  'AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_GAME_CHANGED',
  'AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_GAME_GENERATION_MISMATCH',
  'AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_GAME_IO',
  'AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_GAME_LAYOUT_INVALID',
  'AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_GAME_LIMIT',
  'AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_GAME_PATH_UNSAFE',
  'AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_INPUT_LIMIT',
  'AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_IOSTORE_OPEN_FAILED',
  'AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_PACKAGE_INDEX_FAILED',
  'AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_PLATFORM_UNSUPPORTED',
  'AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_REQUEST_INVALID',
  'AUTHORING_REVISION3_DATAASSET_PACKAGE_INDEX_RESPONSE_LIMIT',
}.contains(code);

bool _revision3InstalledDataAssetInspectionErrorIsRetryable(
  String code,
) => const {
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_CANDIDATE_INVALID',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_EXTRACTION_FAILED',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_GAME_CHANGED',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_GAME_GENERATION_MISMATCH',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_GAME_IO',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_GAME_LAYOUT_INVALID',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_GAME_LIMIT',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_GAME_PATH_UNSAFE',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_INPUT_CHANGED',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_INPUT_LIMIT',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_INPUT_MISSING',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_INPUT_UNAVAILABLE',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_INPUT_UNSAFE',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_INSPECTION_FAILED',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_IOSTORE_OPEN_FAILED',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_PACKAGE_INDEX_FAILED',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_PACKAGE_INDEX_MISMATCH',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_PLATFORM_UNSUPPORTED',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_REQUEST_INVALID',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_RESPONSE_LIMIT',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_SOURCE_SNAPSHOT_MISMATCH',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_USMAP_CHANGED',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_USMAP_IO',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_USMAP_LIMIT',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_USMAP_MISSING',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_INSPECTION_USMAP_UNSAFE',
}.contains(code);

bool _sameRevision3DataAssetSeal(
  AuthoringDraftContentSeal left,
  AuthoringDraftContentSeal right,
) => left.byteLength == right.byteLength && left.sha256 == right.sha256;

bool _revision3NpcPrepareErrorIsRetryable(String code) => const {
  'AUTHORING_REVISION3_NPC_CATALOG_FAILED',
  'AUTHORING_REVISION3_NPC_CATALOG_LIMIT',
  'AUTHORING_REVISION3_NPC_CATALOG_SELECTION_INVALID',
  'AUTHORING_REVISION3_NPC_CATALOG_SELECTION_UNQUALIFIED',
  'AUTHORING_REVISION3_NPC_COLLISION',
  'AUTHORING_REVISION3_NPC_COLLISION_FAILED',
  'AUTHORING_REVISION3_NPC_COLLISION_LIMIT',
  'AUTHORING_REVISION3_NPC_INPUT_CHANGED',
  'AUTHORING_REVISION3_NPC_INPUT_LIMIT',
  'AUTHORING_REVISION3_NPC_INPUT_MISSING',
  'AUTHORING_REVISION3_NPC_INPUT_UNAVAILABLE',
  'AUTHORING_REVISION3_NPC_INPUT_UNSAFE',
  'AUTHORING_REVISION3_NPC_INTENT_INVALID',
  'AUTHORING_REVISION3_NPC_LIMIT',
  'AUTHORING_REVISION3_NPC_PRISTINE_UNAVAILABLE',
  'AUTHORING_REVISION3_NPC_PROJECT_TARGET_MISMATCH',
  'AUTHORING_REVISION3_NPC_RECOVERY_REQUIRED',
  'AUTHORING_REVISION3_NPC_RESPONSE_LIMIT',
  'AUTHORING_REVISION3_NPC_STORE_GAME_ALIAS',
  'AUTHORING_REVISION3_NPC_UNSUPPORTED_GENERATION',
}.contains(code);

bool _revision3NpcProfileEditPrepareErrorIsRetryable(String code) => const {
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
}.contains(code);

bool _revision3DialogLinePrepareErrorIsRetryable(String code) => const {
  'AUTHORING_REVISION3_DIALOG_ENTITY_CONFLICT',
  'AUTHORING_REVISION3_DIALOG_IDENTITY_CONFLICT',
  'AUTHORING_REVISION3_DIALOG_INPUT_LIMIT',
  'AUTHORING_REVISION3_DIALOG_LOCALE_CONFLICT',
  'AUTHORING_REVISION3_DIALOG_LOCALIZATION_CONFLICT',
  'AUTHORING_REVISION3_DIALOG_PROJECT_LIMIT',
  'AUTHORING_REVISION3_DIALOG_REQUEST_LIMIT',
  'AUTHORING_REVISION3_DIALOG_REQUEST_REJECTED',
  'AUTHORING_REVISION3_DIALOG_REVISION_LIMIT',
}.contains(code);

bool _revision3DialogLocalizationReadErrorIsRetryable(String code) => const {
  'AUTHORING_REVISION3_DIALOG_LOCALIZATION_IDENTITY_CONFLICT',
  'AUTHORING_REVISION3_DIALOG_LOCALIZATION_LOCALE_LIMIT',
  'AUTHORING_REVISION3_DIALOG_LOCALIZATION_NOT_FOUND',
  'AUTHORING_REVISION3_DIALOG_LOCALIZATION_RESPONSE_LIMIT',
  'AUTHORING_REVISION3_DIALOG_LOCALIZATION_REVISION_CONFLICT',
}.contains(code);

bool _revision3DialogLocalizationEditReadErrorIsRetryable(String code) =>
    const {
      'AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_BACKLINK_LIMIT',
      'AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_IDENTITY_CONFLICT',
      'AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_LOCALE_LIMIT',
      'AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_NOT_FOUND',
      'AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_ORIGIN_CONFLICT',
      'AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_PROJECT_CONFLICT',
      'AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_RESPONSE_LIMIT',
      'AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_REVISION_CONFLICT',
      'AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_REVISION_LIMIT',
      'AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_SIGNED_WIRE_LIMIT',
      'AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_TARGET_CONFLICT',
      'AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_TEXT_LIMIT',
    }.contains(code);

bool _revision3DialogLocalizationEditPrepareErrorIsRetryable(String code) =>
    const {
      'AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_IDENTITY_CONFLICT',
      'AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_INPUT_LIMIT',
      'AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_LOCALE_LIMIT',
      'AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_NO_CHANGES',
      'AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_NOT_FOUND',
      'AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_ORIGIN_CONFLICT',
      'AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_PROJECT_CONFLICT',
      'AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_PROJECT_LIMIT',
      'AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_REQUEST_INVALID',
      'AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_REQUEST_LIMIT',
      'AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_REQUEST_REJECTED',
      'AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_RESPONSE_LIMIT',
      'AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_REVISION_CONFLICT',
      'AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_REVISION_LIMIT',
      'AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_SIGNED_WIRE_LIMIT',
      'AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_TARGET_CONFLICT',
      'AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_TEXT_LIMIT',
      'AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_VOICE_CONFLICT',
    }.contains(code);

bool _revision3VoicePrepareErrorIsRetryable(String code) => const {
  'AUTHORING_REVISION3_VOICE_GAME_ROOT_UNAVAILABLE',
  'AUTHORING_REVISION3_VOICE_INPUT_CHANGED',
  'AUTHORING_REVISION3_VOICE_INPUT_LIMIT',
  'AUTHORING_REVISION3_VOICE_INPUT_MISSING',
  'AUTHORING_REVISION3_VOICE_INPUT_UNAVAILABLE',
  'AUTHORING_REVISION3_VOICE_INPUT_UNSAFE',
  'AUTHORING_REVISION3_VOICE_INTENT_INVALID',
  'AUTHORING_REVISION3_VOICE_LIMIT',
  'AUTHORING_REVISION3_VOICE_OGG_INVALID',
  'AUTHORING_REVISION3_VOICE_STATUS_INVALID',
  'AUTHORING_REVISION3_VOICE_STORE_GAME_ALIAS',
}.contains(code);

bool _revision3VoicePreviewReadErrorIsRetryable(String code) => const {
  'AUTHORING_REVISION3_VOICE_PREVIEW_LINE_CONFLICT',
  'AUTHORING_REVISION3_VOICE_PREVIEW_LOCALIZATION_CONFLICT',
  'AUTHORING_REVISION3_VOICE_PREVIEW_SLOT_CONFLICT',
  'AUTHORING_REVISION3_VOICE_PREVIEW_TAKE_CONFLICT',
  'AUTHORING_REVISION3_VOICE_PREVIEW_ASSET_CONFLICT',
  'AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_CAPABILITY_INVALID',
  'AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_CAPABILITY_CONFLICT',
  'AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_CAPABILITY_LIMIT',
  'AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_CAPABILITY_UNAVAILABLE',
  'AUTHORING_REVISION3_VOICE_PREVIEW_CLEANUP_TOKEN_UNKNOWN',
  'AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_CAPABILITY_CHANGED',
  'AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_OUTPUT_CONFLICT',
  'AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_IO',
}.contains(code);

bool _revision3VoiceMediaQaReadErrorIsRetryable(String code) => const {
  'AUTHORING_REVISION3_VOICE_MEDIA_LINE_CONFLICT',
  'AUTHORING_REVISION3_VOICE_MEDIA_LOCALIZATION_CONFLICT',
  'AUTHORING_REVISION3_VOICE_MEDIA_SLOT_CONFLICT',
  'AUTHORING_REVISION3_VOICE_MEDIA_TAKE_CONFLICT',
  'AUTHORING_REVISION3_VOICE_MEDIA_ASSET_CONFLICT',
}.contains(code);

bool _revision3VoicePreviewMaterializationFailureRequiresReopen(Object error) {
  if (error is Revision3VoiceTakePreviewMaterializationCleanupException) {
    return _revision3VoicePreviewMaterializationFailureRequiresReopen(
      error.materializationCause,
    );
  }
  if (error is Revision3VoiceTakePreviewCleanupException ||
      error is Revision3VoiceTakePreviewVerificationException ||
      error is ArgumentError ||
      error is FormatException ||
      error is UnsupportedError) {
    return false;
  }
  if (error is ModFfiException) {
    return !_revision3VoicePreviewReadErrorIsRetryable(error.code);
  }
  return true;
}

bool _revision3VoiceBatchPlanErrorIsRetryable(String code) => const {
  'AUTHORING_REVISION3_VOICE_BATCH_REQUEST_INVALID',
  'AUTHORING_REVISION3_VOICE_BATCH_INPUT_LIMIT',
  'AUTHORING_REVISION3_VOICE_BATCH_HEAD_INVALID',
  'AUTHORING_REVISION3_VOICE_BATCH_ROOT_OVERLAP',
  'AUTHORING_REVISION3_VOICE_BATCH_SOURCE_UNAVAILABLE',
  'AUTHORING_REVISION3_VOICE_BATCH_SOURCE_UNSAFE',
  'AUTHORING_REVISION3_VOICE_BATCH_SOURCE_LIMIT',
  'AUTHORING_REVISION3_VOICE_BATCH_SOURCE_CHANGED',
  'AUTHORING_REVISION3_VOICE_BATCH_STORE_MISSING',
  'AUTHORING_REVISION3_VOICE_BATCH_STORE_UNSAFE',
  'AUTHORING_REVISION3_VOICE_BATCH_STORE_LIMIT',
  'AUTHORING_REVISION3_VOICE_BATCH_STORE_IO',
  'AUTHORING_REVISION3_VOICE_BATCH_RESPONSE_LIMIT',
}.contains(code);

bool _revision3VoiceBatchPrepareErrorIsRetryable(String code) =>
    _revision3VoiceBatchPlanErrorIsRetryable(code);

bool _revision3VoiceTargetPrepareErrorIsRetryable(String code) => const {
  'AUTHORING_REVISION3_VOICE_TARGET_ARCHIVE_CHANGED',
  'AUTHORING_REVISION3_VOICE_TARGET_ARCHIVE_INVALID',
  'AUTHORING_REVISION3_VOICE_TARGET_ARCHIVE_LIMIT',
  'AUTHORING_REVISION3_VOICE_TARGET_ARCHIVE_UNAVAILABLE',
  'AUTHORING_REVISION3_VOICE_TARGET_ARCHIVE_UNSAFE',
  'AUTHORING_REVISION3_VOICE_TARGET_COLLISION',
  'AUTHORING_REVISION3_VOICE_TARGET_EXECUTABLE_MISMATCH',
  'AUTHORING_REVISION3_VOICE_TARGET_EXECUTABLE_UNAVAILABLE',
  'AUTHORING_REVISION3_VOICE_TARGET_GAME_ROOT_UNAVAILABLE',
  'AUTHORING_REVISION3_VOICE_TARGET_INPUT_LIMIT',
  'AUTHORING_REVISION3_VOICE_TARGET_INTENT_INVALID',
  'AUTHORING_REVISION3_VOICE_TARGET_LOCALE_UNSUPPORTED',
  'AUTHORING_REVISION3_VOICE_TARGET_LOC_ID_INVALID',
  'AUTHORING_REVISION3_VOICE_TARGET_MEMBER_INELIGIBLE',
  'AUTHORING_REVISION3_VOICE_TARGET_PROJECT_LIMIT',
  'AUTHORING_REVISION3_VOICE_TARGET_REQUEST_INVALID',
  'AUTHORING_REVISION3_VOICE_TARGET_RESPONSE_LIMIT',
  'AUTHORING_REVISION3_VOICE_TARGET_REVISION_LIMIT',
  'AUTHORING_REVISION3_VOICE_TARGET_SIGNED_WIRE_LIMIT',
  'AUTHORING_REVISION3_VOICE_TARGET_STORE_GAME_ALIAS',
}.contains(code);

bool _revision3VoiceSelectionPrepareErrorIsRetryable(String code) => const {
  'AUTHORING_REVISION3_VOICE_SELECTION_INPUT_LIMIT',
  'AUTHORING_REVISION3_VOICE_SELECTION_LINE_CONFLICT',
  'AUTHORING_REVISION3_VOICE_SELECTION_NO_CHANGES',
  'AUTHORING_REVISION3_VOICE_SELECTION_PROJECT_CONFLICT',
  'AUTHORING_REVISION3_VOICE_SELECTION_PROJECT_LIMIT',
  'AUTHORING_REVISION3_VOICE_SELECTION_REQUEST_INVALID',
  'AUTHORING_REVISION3_VOICE_SELECTION_REQUEST_LIMIT',
  'AUTHORING_REVISION3_VOICE_SELECTION_REQUEST_REJECTED',
  'AUTHORING_REVISION3_VOICE_SELECTION_RESPONSE_LIMIT',
  'AUTHORING_REVISION3_VOICE_SELECTION_REVISION_LIMIT',
  'AUTHORING_REVISION3_VOICE_SELECTION_SELECTION_CONFLICT',
  'AUTHORING_REVISION3_VOICE_SELECTION_SIGNED_WIRE_LIMIT',
  'AUTHORING_REVISION3_VOICE_SELECTION_SLOT_CONFLICT',
  'AUTHORING_REVISION3_VOICE_SELECTION_TAKE_CONFLICT',
  'AUTHORING_REVISION3_VOICE_SELECTION_TAKE_NOT_APPROVED',
  'AUTHORING_REVISION3_VOICE_SELECTION_TARGET_CONFLICT',
}.contains(code);

bool _revision3VoiceTakeRemovalPrepareErrorIsRetryable(String code) => const {
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
}.contains(code);

bool _revision3DialogVoiceSlotRemovalPrepareErrorIsRetryable(String code) =>
    const {
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
    }.contains(code);

bool _revision3VoiceTakeStatusPrepareErrorIsRetryable(String code) => const {
  'AUTHORING_REVISION3_VOICE_TAKE_STATUS_INPUT_LIMIT',
  'AUTHORING_REVISION3_VOICE_TAKE_STATUS_LINE_CONFLICT',
  'AUTHORING_REVISION3_VOICE_TAKE_STATUS_NO_CHANGES',
  'AUTHORING_REVISION3_VOICE_TAKE_STATUS_PROJECT_CONFLICT',
  'AUTHORING_REVISION3_VOICE_TAKE_STATUS_PROJECT_LIMIT',
  'AUTHORING_REVISION3_VOICE_TAKE_STATUS_REQUEST_INVALID',
  'AUTHORING_REVISION3_VOICE_TAKE_STATUS_REQUEST_LIMIT',
  'AUTHORING_REVISION3_VOICE_TAKE_STATUS_REQUEST_REJECTED',
  'AUTHORING_REVISION3_VOICE_TAKE_STATUS_RESPONSE_LIMIT',
  'AUTHORING_REVISION3_VOICE_TAKE_STATUS_REVISION_LIMIT',
  'AUTHORING_REVISION3_VOICE_TAKE_STATUS_SELECTED_CONFLICT',
  'AUTHORING_REVISION3_VOICE_TAKE_STATUS_SIGNED_WIRE_LIMIT',
  'AUTHORING_REVISION3_VOICE_TAKE_STATUS_SLOT_CONFLICT',
  'AUTHORING_REVISION3_VOICE_TAKE_STATUS_STATUS_CONFLICT',
  'AUTHORING_REVISION3_VOICE_TAKE_STATUS_TAKE_CONFLICT',
  'AUTHORING_REVISION3_VOICE_TAKE_STATUS_TARGET_CONFLICT',
}.contains(code);

bool _revision3VoiceBuildErrorIsRetryable(String code) => const {
  'AUTHORING_REVISION3_VOICE_BUILD_BUNDLE_INVALID',
  'AUTHORING_REVISION3_VOICE_BUILD_EXECUTABLE_MISMATCH',
  'AUTHORING_REVISION3_VOICE_BUILD_EXECUTABLE_UNAVAILABLE',
  'AUTHORING_REVISION3_VOICE_BUILD_GAME_OUTPUT_ALIAS',
  'AUTHORING_REVISION3_VOICE_BUILD_GAME_ROOT_CHANGED',
  'AUTHORING_REVISION3_VOICE_BUILD_GAME_UNAVAILABLE',
  'AUTHORING_REVISION3_VOICE_BUILD_INPUT_INVALID',
  'AUTHORING_REVISION3_VOICE_BUILD_INPUT_LIMIT',
  'AUTHORING_REVISION3_VOICE_BUILD_CLEANUP_FAILED',
  'AUTHORING_REVISION3_VOICE_BUILD_OUTPUT_FAILED',
  'AUTHORING_REVISION3_VOICE_BUILD_OUTPUT_ROOT_CHANGED',
  'AUTHORING_REVISION3_VOICE_BUILD_OUTPUT_UNAVAILABLE',
  'AUTHORING_REVISION3_VOICE_BUILD_PROMOTION_FAILED',
  'AUTHORING_REVISION3_VOICE_BUILD_RESPONSE_LIMIT',
  'AUTHORING_REVISION3_VOICE_BUILD_STORE_OUTPUT_ALIAS',
  'AUTHORING_REVISION3_VOICE_BUILD_STORE_GAME_ALIAS',
  'AUTHORING_REVISION3_VOICE_BUILD_VERIFY_FAILED',
}.contains(code);

bool _revision3VoiceBuildPlanErrorIsRetryable(String code) => const {
  'AUTHORING_REVISION3_VOICE_PLAN_INPUT_LIMIT',
  'AUTHORING_REVISION3_VOICE_PLAN_PROJECT_INVALID',
  'AUTHORING_REVISION3_VOICE_PLAN_RESPONSE_LIMIT',
}.contains(code);

bool _revision3ExactSnapshotExportErrorIsRetryable(String code) => const {
  'AUTHORING_REVISION3_EXPORT_REQUEST_INVALID',
  'AUTHORING_REVISION3_EXPORT_INPUT_LIMIT',
  'AUTHORING_REVISION3_EXPORT_CLOSURE_LIMIT',
  'AUTHORING_REVISION3_EXPORT_OUTPUT_EXISTS',
  'AUTHORING_REVISION3_EXPORT_OUTPUT_INVALID',
  'AUTHORING_REVISION3_EXPORT_ARCHIVE_FAILED',
  'AUTHORING_REVISION3_EXPORT_VERIFY_FAILED',
  'AUTHORING_REVISION3_EXPORT_CLEANUP_FAILED',
  'AUTHORING_REVISION3_EXPORT_PUBLICATION_FAILED',
}.contains(code);

bool _revision3ExactSnapshotExportErrorIsKnownPrepublication(String code) =>
    const {
      'AUTHORING_REVISION3_EXPORT_HEAD_INVALID',
      'AUTHORING_REVISION3_EXPORT_ROOT_UNAVAILABLE',
      'AUTHORING_REVISION3_EXPORT_STORE_CHANGED',
      'AUTHORING_REVISION3_EXPORT_CLOSURE_INVALID',
      'AUTHORING_REVISION3_EXPORT_INVARIANT',
    }.contains(code);

bool _revision3ReviewedDataAssetBuildErrorIsRetryable(String code) => const {
  'AUTHORING_REVISION3_DATAASSET_BUILD_INPUT_INVALID',
  'AUTHORING_REVISION3_DATAASSET_BUILD_INPUT_LIMIT',
  'AUTHORING_REVISION3_DATAASSET_BUILD_OUTPUT_EXISTS',
  'AUTHORING_REVISION3_DATAASSET_BUILD_OUTPUT_INVALID',
  'AUTHORING_REVISION3_DATAASSET_BUILD_PACK_FAILED',
  'AUTHORING_REVISION3_DATAASSET_BUILD_PACK_NAME_INVALID',
  'AUTHORING_REVISION3_DATAASSET_BUILD_PROJECT_INVALID',
  'AUTHORING_REVISION3_DATAASSET_BUILD_PUBLICATION_FAILED',
  'AUTHORING_REVISION3_DATAASSET_BUILD_RESPONSE_LIMIT',
  'AUTHORING_REVISION3_DATAASSET_BUILD_SOURCE_INVALID',
  'AUTHORING_REVISION3_DATAASSET_BUILD_TARGET_INVALID',
  'AUTHORING_REVISION3_DATAASSET_BUILD_TARGET_MISSING',
  'AUTHORING_REVISION3_DATAASSET_BUILD_TARGET_NOT_REVIEWED',
}.contains(code);

bool _revision3ContentReadErrorRequiresReopen(String code) => const {
  ModFfiException.malformedNativeResponseCode,
  'AUTHORING_REVISION3_CONTENT_HEAD_INVALID',
  'AUTHORING_REVISION3_CONTENT_HEAD_MISSING',
  'AUTHORING_REVISION3_CONTENT_INVARIANT',
  'AUTHORING_REVISION3_CONTENT_STORE_COLLISION',
  'AUTHORING_REVISION3_CONTENT_STORE_INVARIANT',
  'AUTHORING_REVISION3_CONTENT_STORE_IO',
  'AUTHORING_REVISION3_CONTENT_STORE_JSON_INVALID',
  'AUTHORING_REVISION3_CONTENT_STORE_LIMIT',
  'AUTHORING_REVISION3_CONTENT_STORE_LIMITS_INVALID',
  'AUTHORING_REVISION3_CONTENT_STORE_OBJECT_MISSING',
  'AUTHORING_REVISION3_CONTENT_STORE_PATH_UNSAFE',
  'AUTHORING_REVISION3_CONTENT_STORE_ROOT_MISSING',
  'AUTHORING_REVISION3_CONTENT_STORE_SEAL_MISMATCH',
}.contains(code);

bool _revision3HistoryErrorIsRetryable(String code) => const {
  'AUTHORING_REVISION3_HISTORY_REQUEST_INVALID',
  'AUTHORING_REVISION3_HISTORY_INPUT_LIMIT',
  'AUTHORING_REVISION3_HISTORY_HEAD_INVALID',
  'AUTHORING_REVISION3_HISTORY_TARGET_NOT_REACHABLE',
  'AUTHORING_REVISION3_HISTORY_REVISION_LIMIT',
  'AUTHORING_REVISION3_HISTORY_RESPONSE_LIMIT',
}.contains(code);

bool _revision3HistoryErrorNeedsCurrentReverification(String code) =>
    code != 'AUTHORING_REVISION3_HISTORY_HEAD_CONFLICT' &&
    !_revision3HistoryErrorIsRetryable(code);

bool _revision3StoryDraftRemovalErrorIsRetryable(String code) => const {
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
}.contains(code);

bool _revision3DataAssetErrorIsRetryable(String code) => const {
  'AUTHORING_REVISION3_DATAASSET_EDIT_INVALID',
  'AUTHORING_REVISION3_DATAASSET_EXECUTABLE_MISMATCH',
  'AUTHORING_REVISION3_DATAASSET_INPUT_INVALID',
  'AUTHORING_REVISION3_DATAASSET_INPUT_LIMIT',
  'AUTHORING_REVISION3_DATAASSET_INPUT_MISSING',
  'AUTHORING_REVISION3_DATAASSET_INPUT_UNSAFE',
  'AUTHORING_REVISION3_DATAASSET_PROJECT_LIMIT',
  'AUTHORING_REVISION3_DATAASSET_RESPONSE_LIMIT',
  'AUTHORING_REVISION3_DATAASSET_REVISION_LIMIT',
  'AUTHORING_REVISION3_DATAASSET_TARGET_EXISTS',
  'AUTHORING_REVISION3_DATAASSET_TARGET_MISSING',
}.contains(code);

bool _revision3InstalledDataAssetEditErrorIsRetryable(String code) => const {
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_CANDIDATE_INVALID',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_HEAD_INVALID',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_INPUT_LIMIT',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_INSPECTION_BINDING_MISMATCH',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_INSPECTION_FAILED',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_INVALID',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_PACKAGE_INDEX_MISMATCH',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_REQUEST_INVALID',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_RESPONSE_LIMIT',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_REVISION_LIMIT',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_SELECTOR_MISMATCH',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_SOURCE_SNAPSHOT_MISMATCH',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_USMAP_CONTENT_MISMATCH',
  'AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_USMAP_INVENTORY_MISMATCH',
  'AUTHORING_REVISION3_REVIEWED_INSTALLED_DATAASSET_EDIT_CANDIDATE_INVALID',
  'AUTHORING_REVISION3_REVIEWED_INSTALLED_DATAASSET_EDIT_HEAD_INVALID',
  'AUTHORING_REVISION3_REVIEWED_INSTALLED_DATAASSET_EDIT_INPUT_LIMIT',
  'AUTHORING_REVISION3_REVIEWED_INSTALLED_DATAASSET_EDIT_INVALID',
  'AUTHORING_REVISION3_REVIEWED_INSTALLED_DATAASSET_EDIT_MATCH_INVALID',
  'AUTHORING_REVISION3_REVIEWED_INSTALLED_DATAASSET_EDIT_REQUEST_INVALID',
  'AUTHORING_REVISION3_REVIEWED_INSTALLED_DATAASSET_EDIT_RESPONSE_LIMIT',
}.contains(code);

Future<AuthoringWorkingHead> _readCanonicalHead(File file) async {
  final type = await FileSystemEntity.type(file.path, followLinks: false);
  if (type != FileSystemEntityType.file) {
    throw ManagedProjectVerificationException(
      'managed project head is not a regular file: ${file.path}',
    );
  }
  final RandomAccessFile handle;
  try {
    handle = await file.open();
  } on FileSystemException {
    throw ManagedProjectVerificationException(
      'managed project head could not be opened: ${file.path}',
    );
  }
  final Uint8List bytes;
  try {
    final builder = BytesBuilder(copy: false);
    while (builder.length <= _maxManagedHeadBytes) {
      final remaining = _maxManagedHeadBytes + 1 - builder.length;
      final chunk = await handle.read(remaining < 8192 ? remaining : 8192);
      if (chunk.isEmpty) break;
      builder.add(chunk);
    }
    bytes = builder.takeBytes();
  } finally {
    await handle.close();
  }
  if (bytes.isEmpty || bytes.length > _maxManagedHeadBytes) {
    throw ManagedProjectVerificationException(
      'managed project head exceeds its size limit: ${file.path}',
    );
  }
  final String text;
  try {
    text = utf8.decode(bytes, allowMalformed: false);
  } on FormatException {
    throw ManagedProjectVerificationException(
      'managed project head is not valid UTF-8: ${file.path}',
    );
  }
  try {
    return AuthoringWorkingHead.fromCanonicalJson(text);
  } on FormatException {
    throw ManagedProjectVerificationException(
      'managed project head is not canonical: ${file.path}',
    );
  }
}

void _requireExactOpened(
  _ManagedOpenedCheckpoint opened, {
  required AuthoringWorkingHead expectedHead,
  String? expectedProjectJson,
  required String context,
}) {
  if (opened.head.canonicalJson != expectedHead.canonicalJson) {
    throw ManagedProjectVerificationException(
      '$context returned a different head than requested',
    );
  }
  if (expectedProjectJson != null &&
      opened.projectJson != expectedProjectJson) {
    throw ManagedProjectVerificationException(
      '$context did not reproduce the exact captured project JSON',
    );
  }
}

({String projectId, int revision, String canonicalTarget})
_revision3RecoveryIdentity(
  _ManagedOpenedCheckpoint opened, {
  required String context,
}) {
  try {
    final decoded = jsonDecode(opened.projectJson);
    if (decoded is! Map) {
      throw const FormatException('project root is not an object');
    }
    final project = decoded.cast<String, Object?>();
    if (jsonEncode(project) != opened.projectJson ||
        project['format'] != 2 ||
        project['schema_revision'] != 3) {
      throw const FormatException(
        'project is not canonical schema-revision-3 format-2 JSON',
      );
    }
    final projectId = project['project_id'];
    final revision = project['revision'];
    final target = project['target'];
    if (projectId is! String ||
        !_managedRevision3CompilerIdPattern.hasMatch(projectId) ||
        projectId == _managedRevision3CompilerZeroId) {
      throw const FormatException('project ID is not a nonzero entity ID');
    }
    if (revision is! int || revision < 0 || revision > 0x7fffffffffffffff) {
      throw const FormatException(
        'project revision is outside the wire domain',
      );
    }
    if (target is! Map) {
      throw const FormatException('project target is not an object');
    }
    if (opened.projectId != projectId || opened.projectRevision != revision) {
      throw const FormatException(
        'Store checkpoint identity disagrees with its project JSON',
      );
    }
    return (
      projectId: projectId,
      revision: revision,
      canonicalTarget: jsonEncode(target),
    );
  } on ManagedProjectSessionException {
    rethrow;
  } catch (_) {
    throw ManagedProjectVerificationException(
      '$context has invalid or inconsistent project identity',
    );
  }
}

void _requireValidRevision3Recovery({
  required _ManagedOpenedCheckpoint previous,
  required ({String projectId, int revision, String canonicalTarget})
  previousIdentity,
  required _ManagedOpenedCheckpoint recovered,
  required ({String projectId, int revision, String canonicalTarget})
  recoveredIdentity,
}) {
  if (recoveredIdentity.projectId != previousIdentity.projectId ||
      recoveredIdentity.canonicalTarget != previousIdentity.canonicalTarget) {
    throw const ManagedProjectVerificationException(
      'recovered revision-3 checkpoint changed project identity or target',
    );
  }

  if (recoveredIdentity.revision == previousIdentity.revision) {
    if (recovered.head.canonicalJson != previous.head.canonicalJson ||
        recovered.projectJson != previous.projectJson) {
      throw const ManagedProjectVerificationException(
        'same-revision recovery did not reproduce the exact prior checkpoint',
      );
    }
    return;
  }

  if (previousIdentity.revision == 0x7fffffffffffffff ||
      recoveredIdentity.revision != previousIdentity.revision + 1 ||
      recovered.head.canonicalJson == previous.head.canonicalJson) {
    throw const ManagedProjectVerificationException(
      'recovered revision-3 checkpoint is not the prior or next generation',
    );
  }
}
