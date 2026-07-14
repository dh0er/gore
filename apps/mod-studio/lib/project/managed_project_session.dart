import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:path/path.dart' as p;

import '../core/mod_ffi.dart';
import 'managed_project_lock.dart';
import 'project_atomic_io.dart';
import 'revision3_content_index.dart';

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

/// One imported VoiceTake returned only after its native candidate was fully reopened,
/// fixed-head CAS published, and fully reopened again. The slot remains target-unresolved and
/// this value grants no archive-member, build, runtime, deployment, or native-publication claim.
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

  Future<AuthoringRevision3NpcDraftPreparation> prepareNpcDraftV1({
    required String root,
    required String gameRoot,
    required String currentProjectJson,
    required AuthoringRevision3NpcDraftRequestV1 request,
  });

  Future<AuthoringRevision3VoiceTakePreparation> prepareVoiceTakeV1({
    required String root,
    required String gameRoot,
    required String source,
    required String currentProjectJson,
    required AuthoringRevision3VoiceTakeRequestV1 request,
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

final class ModFfiManagedRevision3AuthoringStore
    implements ManagedRevision3AuthoringStore {
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

abstract interface class _ManagedCheckpointStore {
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
}

final class _Revision12ManagedCheckpointStore
    implements _ManagedCheckpointStore {
  const _Revision12ManagedCheckpointStore(this.store, this.profile);

  final ManagedAuthoringStore store;
  final AuthoringValidationProfile profile;

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

  Future<ManagedRevision3DataAssetStageCheckpoint>
  _prepareAndPublishDataAssetStage({
    required String operation,
    required Future<AuthoringRevision3DataAssetStagePreparation> Function(
      _ManagedOpenedCheckpoint basis,
    )
    prepare,
  }) =>
      _core._publishPreparedRevision3Checkpoint<
        ManagedRevision3DataAssetStageCheckpoint
      >(
        operation: operation,
        handlePrepareError: _core._throwRevision3DataAssetError,
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

  Future<void> close() => _core.close();
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

  /// True after an I/O or verification failure leaves publication state
  /// uncertain. Close and reopen before attempting another edit.
  bool get requiresReopen => _requiresReopen;

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
  /// the session so callers must close and reopen before another edit.
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
        'managed project must be reopened after an uncertain publication',
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
