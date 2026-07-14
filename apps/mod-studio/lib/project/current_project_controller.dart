import 'dart:async';
import 'dart:io';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart';

import '../core/mod_ffi.dart';
import '../core/providers.dart';
import 'managed_project_session.dart';
import 'project_controller.dart';
import 'revision3_content_index.dart';
import 'revision3_dataasset_authoring.dart';
import 'revision3_npc_authoring.dart';
import 'revision3_quest_authoring.dart';
import 'revision3_quest_context_authoring.dart';
import 'revision3_quest_outline_authoring.dart';
import 'revision3_quest_transitions_authoring.dart';
import 'revision3_voice_authoring.dart';
import 'revision3_voice_take_selection_authoring.dart';

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

final class Revision3DataAssetPackageIndexRequiresReopenException
    implements Exception {
  const Revision3DataAssetPackageIndexRequiresReopenException();
}

final class Revision3DataAssetPackageIndexStaleCheckpointException
    implements Exception {
  const Revision3DataAssetPackageIndexStaleCheckpointException();
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
  Future<AuthoringRevision3DataAssetPackageIndexResult>
  readDataAssetPackageIndexV1({required String gameRoot});
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
  Future<Revision3DataAssetStageRemovalPublication>
  prepareAndPublishRemoveDataAssetStageV1({required String targetPath});
  Future<void> verifyCurrentHead();
  Future<void> close();
}

typedef ManagedRevision3CurrentProjectOpener =
    Future<ManagedRevision3CurrentProjectLease> Function(Directory root);

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
    implements ManagedRevision3CurrentProjectLease {
  const _ManagedRevision3SessionLease(this._session);

  final ManagedRevision3AuthoringProjectSession _session;

  @override
  AuthoringWorkingHead get head => _session.head;

  @override
  String get projectId => _session.projectId;

  @override
  int get projectRevision => _session.projectRevision;

  @override
  bool get requiresReopen => _session.requiresReopen;

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
  Future<AuthoringRevision3DataAssetPackageIndexResult>
  readDataAssetPackageIndexV1({required String gameRoot}) =>
      _session.readDataAssetPackageIndexV1(gameRoot: gameRoot);

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
    final checkpoint = await _session.prepareAndPublishQuestOutlineEditV1(
      questId: input.questId,
      expectedQuestRevision: input.expectedQuestRevision,
      expectedModuleId: input.moduleId,
      expectedModuleRevision: input.expectedModuleRevision,
      displayName: input.displayName,
      title: input.title,
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

final currentProjectCoordinatorProvider =
    StateNotifierProvider<CurrentProjectCoordinator, CurrentProjectState>((
      ref,
    ) {
      final createLegacy = ref.read(legacyCurrentProjectLeaseFactoryProvider);
      return CurrentProjectCoordinator(
        initialLegacy: createLegacy(),
        createLegacy: createLegacy,
        openManagedRevision3: ref.read(
          managedRevision3CurrentProjectOpenerProvider,
        ),
      );
    });

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
      openManagedRevision3: openManagedRevision3,
    );
  }

  CurrentProjectCoordinator._({
    required this._current,
    required CurrentProjectState initialState,
    required this._createLegacy,
    required this._openManagedRevision3,
  }) : super(initialState);

  final LegacyCurrentProjectLeaseFactory? _createLegacy;
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
