import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/managed_project_session.dart';
import 'package:gore_mod/project/project_atomic_io.dart';
import 'package:gore_mod/project/revision3_project_history.dart';
import 'package:path/path.dart' as p;

import '../support/revision3_voice_fixture.dart';

void main() {
  late Directory fixture;

  setUp(() async {
    fixture = await Directory.systemTemp.createTemp('gore_managed_session_');
  });

  tearDown(() async {
    if (await fixture.exists()) await fixture.delete(recursive: true);
  });

  test(
    'reviewed DataAsset build returns its sealed basis receipt and poisons a post-audit drift',
    () async {
      final root = Directory(p.join(fixture.path, 'reviewed-build-project'));
      await root.create();
      final store = _FakeRevision3ReviewedBuildStore();
      final project = revision3VoiceFixtureProjectJson(revision: 7);
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: project,
      );
      final basisHead = session.head;
      store.afterBuild = (buildRoot, result) async {
        final driftProject = revision3VoiceFixtureProjectJson(revision: 8);
        final driftHead = store.register(driftProject);
        await File(
          p.join(buildRoot, 'gore-project.json'),
        ).writeAsString(driftHead.canonicalJson, flush: true);
        expect(result.basisHead.canonicalJson, basisHead.canonicalJson);
      };

      const targetPath = '/Game/Blueprints/Items/FootstepPreset';
      const packName = 'ReviewedFootsteps';
      const output = r'C:\Builds\ReviewedFootsteps';
      final result = await session.buildReviewedDataAssetV1(
        gameRoot: r'C:\Games\Gothic Remake',
        targetPath: targetPath,
        packName: packName,
        output: output,
      );

      expect(result.basisHead.canonicalJson, basisHead.canonicalJson);
      expect(
        result.receipt.relativeName,
        'gore-authoring-dataasset-build.json',
      );
      expect(result.receipt.sha256, List<String>.filled(64, 'd').join());
      expect(result.output, output);
      expect(store.buildCalls, 1);
      expect(store.buildCurrentProjects.single, project);
      expect(store.buildHeads.single.canonicalJson, basisHead.canonicalJson);
      expect(session.requiresReopen, isTrue);
      await expectLater(
        session.buildReviewedDataAssetV1(
          gameRoot: r'C:\Games\Gothic Remake',
          targetPath: targetPath,
          packName: packName,
          output: r'C:\Builds\MustNotRetry',
        ),
        throwsA(isA<ManagedProjectVerificationException>()),
      );
      expect(store.buildCalls, 1);
      await session.close();
    },
  );

  test(
    'checkpoint-only revision-3 Store rejects reviewed build without poisoning the session',
    () async {
      final root = Directory(p.join(fixture.path, 'checkpoint-only-project'));
      await root.create();
      final buildStore = _FakeRevision3ReviewedBuildStore();
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: _FakeRevision3CheckpointOnlyStore(buildStore),
        projectJson: revision3VoiceFixtureProjectJson(revision: 7),
      );

      expect(session.supportsReviewedDataAssetBuild, isFalse);
      await expectLater(
        session.buildReviewedDataAssetV1(
          gameRoot: r'C:\Games\Gothic Remake',
          targetPath: '/Game/Blueprints/Items/FootstepPreset',
          packName: 'ReviewedFootsteps',
          output: r'C:\Builds\ReviewedFootsteps',
        ),
        throwsA(isA<UnsupportedError>()),
      );
      expect(buildStore.buildCalls, 0);
      expect(session.requiresReopen, isFalse);
      await session.verifyCurrentHead();
      expect(session.requiresReopen, isFalse);
      await session.close();
    },
  );

  test(
    'revision-3 history read is exact-basis and maps the complete truncated window',
    () async {
      final root = Directory(p.join(fixture.path, 'history-read-project'));
      await root.create();
      final store = _FakeRevision3HistoryStore();
      final currentProject = revision3VoiceFixtureProjectJson(revision: 300);
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: currentProject,
      );
      final currentHead = session.head;
      final priorHeads = <AuthoringWorkingHead>[
        for (var revision = 299; revision >= 45; revision--)
          store.register(revision3VoiceFixtureProjectJson(revision: revision)),
      ];
      store.configureHistory(
        currentHead: currentHead,
        priorHeads: priorHeads,
        historyTruncated: true,
      );

      final history = await session.readProjectHistoryV1();

      expect(session.supportsProjectHistory, isTrue);
      expect(store.historyListCalls, 1);
      expect(store.historyListRoots.single, root.path);
      expect(
        store.historyListExpectedHeads.single.canonicalJson,
        currentHead.canonicalJson,
      );
      expect(history.basisHead.canonicalJson, currentHead.canonicalJson);
      expect(history.projectId, revision3VoiceFixtureProjectId);
      expect(history.currentRevision, 300);
      expect(history.historyTruncated, isTrue);
      expect(history.entries, hasLength(256));
      expect(history.entries.first.isCurrent, isTrue);
      expect(history.entries.first.projectRevision, 300);
      expect(history.entries[1].isCurrent, isFalse);
      expect(history.entries[1].projectRevision, 299);
      expect(history.earliestVisibleRevision, 45);
      expect(session.head.canonicalJson, currentHead.canonicalJson);
      expect(session.projectJson, currentProject);
      expect(await session.headFile.readAsString(), currentHead.canonicalJson);
      expect(session.requiresReopen, isFalse);
      await session.close();
    },
  );

  test(
    'missing retained history stays local when the exact current fully reopens',
    () async {
      final root = Directory(
        p.join(fixture.path, 'history-retained-object-missing-project'),
      );
      await root.create();
      final store = _FakeRevision3HistoryStore();
      final currentProject = revision3VoiceFixtureProjectJson(revision: 7);
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: currentProject,
      );
      final currentHead = session.head;
      final priorHead = store.register(
        revision3VoiceFixtureProjectJson(revision: 6),
      );
      store.configureHistory(
        currentHead: currentHead,
        priorHeads: <AuthoringWorkingHead>[priorHead],
      );
      store.nextHistoryListError = const ModFfiException(
        command: 'authoring_store_list_revision3_history_v1',
        code: 'AUTHORING_REVISION3_HISTORY_STORE_OBJECT_MISSING',
        message: 'old retained snapshot is missing',
      );
      final openCallsBefore = store.openCalls;

      await expectLater(
        session.readProjectHistoryV1(),
        throwsA(
          isA<ModFfiException>().having(
            (error) => error.code,
            'code',
            'AUTHORING_REVISION3_HISTORY_STORE_OBJECT_MISSING',
          ),
        ),
      );

      expect(store.openCalls, openCallsBefore + 1);
      expect(store.openVerifications.last, AuthoringAssetVerification.full);
      expect(session.head.canonicalJson, currentHead.canonicalJson);
      expect(session.projectJson, currentProject);
      expect(session.requiresReopen, isFalse);
      await session.verifyCurrentHead();
      expect(session.requiresReopen, isFalse);
      await session.close();
    },
  );

  test(
    'ambiguous history failure poisons when the exact current cannot reopen',
    () async {
      final root = Directory(
        p.join(fixture.path, 'history-current-object-missing-project'),
      );
      await root.create();
      final store = _FakeRevision3HistoryStore();
      final currentProject = revision3VoiceFixtureProjectJson(revision: 7);
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: currentProject,
      );
      final currentHead = session.head;
      final priorHead = store.register(
        revision3VoiceFixtureProjectJson(revision: 6),
      );
      store.configureHistory(
        currentHead: currentHead,
        priorHeads: <AuthoringWorkingHead>[priorHead],
      );
      store.nextHistoryListError = const ModFfiException(
        command: 'authoring_store_list_revision3_history_v1',
        code: 'AUTHORING_REVISION3_HISTORY_STORE_OBJECT_MISSING',
        message: 'snapshot object is missing',
      );
      store.nextOpenError = StateError('current checkpoint cannot fully open');

      await expectLater(
        session.readProjectHistoryV1(),
        throwsA(isA<ManagedProjectVerificationException>()),
      );

      expect(store.openVerifications.last, AuthoringAssetVerification.full);
      expect(session.head.canonicalJson, currentHead.canonicalJson);
      expect(session.projectJson, currentProject);
      expect(session.requiresReopen, isTrue);
      await session.close();
    },
  );

  test(
    'missing retained restore payload stays local when current fully reopens',
    () async {
      final root = Directory(
        p.join(fixture.path, 'history-restore-payload-missing-project'),
      );
      await root.create();
      final store = _FakeRevision3HistoryStore();
      final currentProject = revision3VoiceFixtureProjectJson(revision: 7);
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: currentProject,
      );
      final currentHead = session.head;
      final priorHead = store.register(
        revision3VoiceFixtureProjectJson(revision: 6),
      );
      store.configureHistory(
        currentHead: currentHead,
        priorHeads: <AuthoringWorkingHead>[priorHead],
      );
      final history = await session.readProjectHistoryV1();
      store.nextHistoryRestoreError = const ModFfiException(
        command: 'authoring_store_prepare_revision3_history_restore_v1',
        code: 'AUTHORING_REVISION3_HISTORY_STORE_OBJECT_MISSING',
        message: 'retained checkpoint payload is missing',
      );
      final openCallsBefore = store.openCalls;

      await expectLater(
        session.prepareAndPublishProjectHistoryRestoreV1(
          expectedHistory: history,
          target: history.entries[1],
        ),
        throwsA(
          isA<ModFfiException>().having(
            (error) => error.code,
            'code',
            'AUTHORING_REVISION3_HISTORY_STORE_OBJECT_MISSING',
          ),
        ),
      );

      expect(store.historyRestorePrepareCalls, 1);
      expect(store.openCalls, openCallsBefore + 1);
      expect(store.openVerifications.last, AuthoringAssetVerification.full);
      expect(await session.headFile.readAsString(), currentHead.canonicalJson);
      expect(session.head.canonicalJson, currentHead.canonicalJson);
      expect(session.projectJson, currentProject);
      expect(session.requiresReopen, isFalse);
      await session.close();
    },
  );

  test(
    'revision-3 history restore fully reopens prepares CAS-publishes and reopens published',
    () async {
      final root = Directory(p.join(fixture.path, 'history-restore-project'));
      await root.create();
      final store = _FakeRevision3HistoryStore();
      final currentProject = revision3VoiceFixtureProjectJson(revision: 7);
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: currentProject,
      );
      final previousHead = session.head;
      final revision6Head = store.register(
        revision3VoiceFixtureProjectJson(revision: 6),
      );
      final revision5Project = revision3VoiceFixtureProjectJson(revision: 5);
      final revision5Head = store.register(revision5Project);
      store.configureHistory(
        currentHead: previousHead,
        priorHeads: <AuthoringWorkingHead>[revision6Head, revision5Head],
      );
      final history = await session.readProjectHistoryV1();
      final target = history.entries.last;
      final openCallsBefore = store.openCalls;
      final headOpenCallsBefore = store.openHeadRequests.length;

      final restored = await session.prepareAndPublishProjectHistoryRestoreV1(
        expectedHistory: history,
        target: target,
      );

      final candidateHead = store.lastHistoryRestoreHead!;
      final candidateProject = store.lastHistoryRestoreProjectJson!;
      expect(store.historyRestorePrepareCalls, 1);
      expect(
        store.historyRestoreExpectedHeads.single.canonicalJson,
        previousHead.canonicalJson,
      );
      expect(
        store.historyRestoreTargetHeads.single.canonicalJson,
        revision5Head.canonicalJson,
      );
      expect(
        store.fixedHeadObservedAfterHistoryPrepare,
        previousHead.canonicalJson,
        reason: 'native history restore remains prepare-only',
      );
      final restoreHeadOpens = store.openHeadRequests.skip(headOpenCallsBefore);
      expect(restoreHeadOpens, isNotEmpty);
      expect(
        restoreHeadOpens.map((head) => head.canonicalJson),
        everyElement(candidateHead.canonicalJson),
      );
      expect(
        store.openHeadVerifications.skip(headOpenCallsBefore),
        everyElement(AuthoringAssetVerification.full),
      );
      expect(store.openCalls, openCallsBefore + 1);
      expect(store.openVerifications.last, AuthoringAssetVerification.full);
      expect(
        await session.headFile.readAsString(),
        candidateHead.canonicalJson,
      );
      expect(session.head.canonicalJson, candidateHead.canonicalJson);
      expect(session.projectJson, candidateProject);
      expect(session.projectRevision, 8);
      expect(restored.previousHead.canonicalJson, previousHead.canonicalJson);
      expect(restored.head.canonicalJson, candidateHead.canonicalJson);
      expect(restored.projectJson, candidateProject);
      expect(restored.projectId, revision3VoiceFixtureProjectId);
      expect(restored.previousProjectRevision, 7);
      expect(restored.projectRevision, 8);
      expect(
        restored.restoredFromHead.canonicalJson,
        revision5Head.canonicalJson,
      );
      expect(restored.restoredFromRevision, 5);
      expect(session.requiresReopen, isFalse);
      await session.close();
    },
  );

  test(
    'stale history and foreign restore targets never reach native prepare',
    () async {
      final root = Directory(
        p.join(fixture.path, 'history-local-reject-project'),
      );
      await root.create();
      final store = _FakeRevision3HistoryStore();
      final currentProject = revision3VoiceFixtureProjectJson(revision: 7);
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: currentProject,
      );
      final revision6Head = store.register(
        revision3VoiceFixtureProjectJson(revision: 6),
      );
      store.configureHistory(
        currentHead: session.head,
        priorHeads: <AuthoringWorkingHead>[revision6Head],
      );
      final history = await session.readProjectHistoryV1();
      final fixedHead = session.head.canonicalJson;
      final staleCurrentHead = store.register(currentProject);
      final staleHistory = Revision3ProjectHistorySnapshot(
        basisHead: staleCurrentHead,
        projectId: revision3VoiceFixtureProjectId,
        currentRevision: 7,
        entries: <Revision3ProjectHistoryEntry>[
          Revision3ProjectHistoryEntry(
            head: staleCurrentHead,
            projectId: revision3VoiceFixtureProjectId,
            projectRevision: 7,
            isCurrent: true,
          ),
          history.entries[1],
        ],
        historyTruncated: false,
      );

      await expectLater(
        session.prepareAndPublishProjectHistoryRestoreV1(
          expectedHistory: staleHistory,
          target: staleHistory.entries[1],
        ),
        throwsA(isA<FormatException>()),
      );

      final foreignTarget = Revision3ProjectHistoryEntry(
        head: store.register(revision3VoiceFixtureProjectJson(revision: 6)),
        projectId: List<String>.filled(32, 'f').join(),
        projectRevision: 6,
        isCurrent: false,
      );
      await expectLater(
        session.prepareAndPublishProjectHistoryRestoreV1(
          expectedHistory: history,
          target: foreignTarget,
        ),
        throwsA(isA<FormatException>()),
      );

      final currentMarkedTarget = Revision3ProjectHistoryEntry(
        head: history.entries[1].head,
        projectId: history.projectId,
        projectRevision: history.entries[1].projectRevision,
        isCurrent: true,
      );
      await expectLater(
        session.prepareAndPublishProjectHistoryRestoreV1(
          expectedHistory: history,
          target: currentMarkedTarget,
        ),
        throwsA(isA<FormatException>()),
      );

      expect(store.historyRestorePrepareCalls, 0);
      expect(await session.headFile.readAsString(), fixedHead);
      expect(session.head.canonicalJson, fixedHead);
      expect(session.projectJson, currentProject);
      expect(session.requiresReopen, isFalse);
      await session.close();
    },
  );

  test(
    'uncertain history restore publication poisons after candidate reopen',
    () async {
      final root = Directory(p.join(fixture.path, 'history-uncertain-project'));
      await root.create();
      final store = _FakeRevision3HistoryStore();
      var operationSequence = 0;
      var publicationArmed = false;
      final replacement = AtomicByteReplacement(
        operationIdFactory: () =>
            (++operationSequence).toString().padLeft(32, '0'),
        onPhase: (phase) {
          if (publicationArmed && phase == AtomicSwapPhase.tempPromoted) {
            throw const _InjectedCrash();
          }
        },
      );
      final currentProject = revision3VoiceFixtureProjectJson(revision: 7);
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: currentProject,
        replacement: replacement,
      );
      final previousHead = session.head;
      final targetHead = store.register(
        revision3VoiceFixtureProjectJson(revision: 6),
      );
      store.configureHistory(
        currentHead: previousHead,
        priorHeads: <AuthoringWorkingHead>[targetHead],
      );
      final history = await session.readProjectHistoryV1();
      final openCallsBefore = store.openCalls;
      final headOpenCallsBefore = store.openHeadRequests.length;
      publicationArmed = true;

      await expectLater(
        session.prepareAndPublishProjectHistoryRestoreV1(
          expectedHistory: history,
          target: history.entries[1],
        ),
        throwsA(isA<_InjectedCrash>()),
      );

      final candidateHead = store.lastHistoryRestoreHead!;
      expect(store.historyRestorePrepareCalls, 1);
      final restoreHeadOpens = store.openHeadRequests.skip(headOpenCallsBefore);
      expect(restoreHeadOpens, isNotEmpty);
      expect(
        restoreHeadOpens.map((head) => head.canonicalJson),
        everyElement(candidateHead.canonicalJson),
      );
      expect(store.openCalls, openCallsBefore);
      expect(
        await session.headFile.readAsString(),
        candidateHead.canonicalJson,
      );
      expect(session.head.canonicalJson, previousHead.canonicalJson);
      expect(session.projectJson, currentProject);
      expect(session.requiresReopen, isTrue);
      await expectLater(
        session.prepareAndPublishProjectHistoryRestoreV1(
          expectedHistory: history,
          target: history.entries[1],
        ),
        throwsA(isA<ManagedProjectVerificationException>()),
      );
      expect(store.historyRestorePrepareCalls, 1);
      await session.close();
    },
  );
}

typedef _AfterRevision3ReviewedBuild =
    FutureOr<void> Function(
      String root,
      AuthoringRevision3ReviewedDataAssetBuildResult result,
    );

final class _FakeRevision3CheckpointOnlyStore
    implements ManagedRevision3AuthoringStore {
  const _FakeRevision3CheckpointOnlyStore(this.delegate);

  final _FakeRevision3ReviewedBuildStore delegate;

  @override
  Future<AuthoringRevision3StoreOpenedResult> open({
    required String root,
    required AuthoringAssetVerification verification,
  }) => delegate.open(root: root, verification: verification);

  @override
  Future<AuthoringRevision3StoreOpenedResult> openHeadBytes({
    required String root,
    required AuthoringWorkingHead head,
    required AuthoringAssetVerification verification,
  }) => delegate.openHeadBytes(
    root: root,
    head: head,
    verification: verification,
  );

  @override
  Future<AuthoringRevision3CheckpointPreparation> prepareCheckpoint({
    required String root,
    required AuthoringWorkingHead? expectedHead,
    required String projectJson,
  }) => delegate.prepareCheckpoint(
    root: root,
    expectedHead: expectedHead,
    projectJson: projectJson,
  );

  @override
  dynamic noSuchMethod(Invocation invocation) => throw UnsupportedError(
    'unexpected checkpoint-only revision-3 Store call: ${invocation.memberName}',
  );
}

final class _FakeRevision3HistoryStore
    implements
        ManagedRevision3AuthoringStore,
        ManagedRevision3ProjectHistoryStore {
  final _FakeRevision3ReviewedBuildStore _delegate =
      _FakeRevision3ReviewedBuildStore();
  List<AuthoringWorkingHead> _historyHeads = <AuthoringWorkingHead>[];
  bool _historyTruncated = false;
  ModFfiException? nextHistoryListError;
  ModFfiException? nextHistoryRestoreError;
  Object? nextOpenError;

  int historyListCalls = 0;
  int historyRestorePrepareCalls = 0;
  int openCalls = 0;
  final List<String> historyListRoots = <String>[];
  final List<AuthoringWorkingHead> historyListExpectedHeads =
      <AuthoringWorkingHead>[];
  final List<AuthoringWorkingHead> historyRestoreExpectedHeads =
      <AuthoringWorkingHead>[];
  final List<AuthoringWorkingHead> historyRestoreTargetHeads =
      <AuthoringWorkingHead>[];
  final List<AuthoringAssetVerification> openVerifications =
      <AuthoringAssetVerification>[];
  final List<AuthoringWorkingHead> openHeadRequests = <AuthoringWorkingHead>[];
  final List<AuthoringAssetVerification> openHeadVerifications =
      <AuthoringAssetVerification>[];
  AuthoringWorkingHead? lastHistoryRestoreHead;
  String? lastHistoryRestoreProjectJson;
  String? fixedHeadObservedAfterHistoryPrepare;

  AuthoringWorkingHead register(String projectJson) =>
      _delegate.register(projectJson);

  void configureHistory({
    required AuthoringWorkingHead currentHead,
    required List<AuthoringWorkingHead> priorHeads,
    bool historyTruncated = false,
  }) {
    _historyHeads = List<AuthoringWorkingHead>.unmodifiable(
      <AuthoringWorkingHead>[currentHead, ...priorHeads],
    );
    _historyTruncated = historyTruncated;
  }

  @override
  Future<AuthoringRevision3StoreOpenedResult> open({
    required String root,
    required AuthoringAssetVerification verification,
  }) {
    openCalls++;
    openVerifications.add(verification);
    final error = nextOpenError;
    nextOpenError = null;
    if (error != null) throw error;
    return _delegate.open(root: root, verification: verification);
  }

  @override
  Future<AuthoringRevision3StoreOpenedResult> openHeadBytes({
    required String root,
    required AuthoringWorkingHead head,
    required AuthoringAssetVerification verification,
  }) {
    openHeadRequests.add(head);
    openHeadVerifications.add(verification);
    return _delegate.openHeadBytes(
      root: root,
      head: head,
      verification: verification,
    );
  }

  @override
  Future<AuthoringRevision3CheckpointPreparation> prepareCheckpoint({
    required String root,
    required AuthoringWorkingHead? expectedHead,
    required String projectJson,
  }) => _delegate.prepareCheckpoint(
    root: root,
    expectedHead: expectedHead,
    projectJson: projectJson,
  );

  @override
  Future<AuthoringRevision3ProjectHistoryResult> listProjectHistoryV1({
    required String root,
    required AuthoringWorkingHead expectedHead,
  }) async {
    historyListCalls++;
    historyListRoots.add(root);
    historyListExpectedHeads.add(expectedHead);
    final fixedHead = await File(
      p.join(root, 'gore-project.json'),
    ).readAsString();
    if (fixedHead != expectedHead.canonicalJson ||
        _historyHeads.isEmpty ||
        _historyHeads.first.canonicalJson != expectedHead.canonicalJson) {
      throw const ModFfiException(
        command: 'authoring_store_list_revision3_history_v1',
        code: 'AUTHORING_REVISION3_HISTORY_HEAD_CONFLICT',
        message: 'fake history basis CAS rejected',
      );
    }
    final failure = nextHistoryListError;
    nextHistoryListError = null;
    if (failure != null) throw failure;
    final currentProject = _projectFor(expectedHead);
    final current = jsonDecode(currentProject) as Map<String, dynamic>;
    return AuthoringRevision3ProjectHistoryResult.fromJson(<String, Object?>{
      'ok': true,
      'outcome': 'listed_exact_current',
      'basis_head_json': expectedHead.canonicalJson,
      'project_id': current['project_id'],
      'project_revision': current['revision'],
      'entries': <Object?>[
        for (var index = 0; index < _historyHeads.length; index++)
          _historyEntry(_historyHeads[index], current: index == 0),
      ],
      'history_truncated': _historyTruncated,
      'history_authority': 'authenticated_bounded_history',
      'project_mutation': 'not_performed',
      'game_mutation': 'not_performed',
      'save_mutation': 'not_performed',
      'build_status': 'not_performed',
      'deployment_status': 'not_performed',
      'runtime_status': 'runtime_unqualified',
      'publication_status': 'not_applicable',
    }, expectedHead: expectedHead);
  }

  @override
  Future<AuthoringRevision3ProjectHistoryRestorePreparation>
  prepareProjectHistoryRestoreV1({
    required String root,
    required AuthoringWorkingHead expectedHead,
    required AuthoringWorkingHead targetHead,
  }) async {
    historyRestorePrepareCalls++;
    historyRestoreExpectedHeads.add(expectedHead);
    historyRestoreTargetHeads.add(targetHead);
    final fixedHeadFile = File(p.join(root, 'gore-project.json'));
    final fixedHead = await fixedHeadFile.readAsString();
    final targetRetained = _historyHeads
        .skip(1)
        .any((head) => head.canonicalJson == targetHead.canonicalJson);
    if (fixedHead != expectedHead.canonicalJson ||
        _historyHeads.isEmpty ||
        _historyHeads.first.canonicalJson != expectedHead.canonicalJson) {
      throw const ModFfiException(
        command: 'authoring_store_prepare_revision3_history_restore_v1',
        code: 'AUTHORING_REVISION3_HISTORY_HEAD_CONFLICT',
        message: 'fake history restore basis CAS rejected',
      );
    }
    if (!targetRetained) {
      throw const ModFfiException(
        command: 'authoring_store_prepare_revision3_history_restore_v1',
        code: 'AUTHORING_REVISION3_HISTORY_TARGET_NOT_REACHABLE',
        message: 'fake history target is not retained',
      );
    }
    final failure = nextHistoryRestoreError;
    nextHistoryRestoreError = null;
    if (failure != null) throw failure;
    final current =
        jsonDecode(_projectFor(expectedHead)) as Map<String, dynamic>;
    final historicalProject = _projectFor(targetHead);
    final historical = jsonDecode(historicalProject) as Map<String, dynamic>;
    final candidate = Map<String, dynamic>.from(historical);
    candidate['revision'] = (current['revision'] as int) + 1;
    final candidateProjectJson = jsonEncode(candidate);
    final candidateHead = register(candidateProjectJson);
    lastHistoryRestoreHead = candidateHead;
    lastHistoryRestoreProjectJson = candidateProjectJson;
    fixedHeadObservedAfterHistoryPrepare = await fixedHeadFile.readAsString();
    return AuthoringRevision3ProjectHistoryRestorePreparation.fromJson(
      <String, Object?>{
        'ok': true,
        'outcome': 'prepared_restore_unpublished',
        'basis_head_json': expectedHead.canonicalJson,
        'direct_parent_head_json': expectedHead.canonicalJson,
        'restored_from_head_json': targetHead.canonicalJson,
        'head_json': candidateHead.canonicalJson,
        'project_json': candidateProjectJson,
        'project_id': current['project_id'],
        'previous_project_revision': current['revision'],
        'revision': candidate['revision'],
        'restored_from_revision': historical['revision'],
        'history_authority': 'authenticated_bounded_history',
        'project_mutation': 'prepared_not_published',
        'game_mutation': 'not_performed',
        'save_mutation': 'not_performed',
        'build_status': 'not_performed',
        'deployment_status': 'not_performed',
        'runtime_status': 'runtime_unqualified',
        'publication_status': 'not_supported',
      },
      expectedHead: expectedHead,
      targetHead: targetHead,
    );
  }

  String _projectFor(AuthoringWorkingHead head) {
    final project = _delegate._projectsByHead[head.canonicalJson];
    if (project == null) throw StateError('unknown fake history checkpoint');
    return project;
  }

  Map<String, Object?> _historyEntry(
    AuthoringWorkingHead head, {
    required bool current,
  }) {
    final project = jsonDecode(_projectFor(head)) as Map<String, dynamic>;
    return <String, Object?>{
      'head_json': head.canonicalJson,
      'project_id': project['project_id'],
      'project_revision': project['revision'],
      'current': current,
    };
  }

  @override
  dynamic noSuchMethod(Invocation invocation) => throw UnsupportedError(
    'unexpected fake revision-3 history Store call: ${invocation.memberName}',
  );
}

final class _FakeRevision3ReviewedBuildStore
    implements
        ManagedRevision3AuthoringStore,
        ManagedRevision3ReviewedDataAssetBuildStore {
  final Map<String, String> _projectsByHead = <String, String>{};
  int _sequence = 0;
  int buildCalls = 0;
  final List<String> buildCurrentProjects = <String>[];
  final List<AuthoringWorkingHead> buildHeads = <AuthoringWorkingHead>[];
  _AfterRevision3ReviewedBuild? afterBuild;

  AuthoringWorkingHead register(String projectJson) {
    _sequence++;
    final head = AuthoringWorkingHead.fromCanonicalJson(
      jsonEncode(<String, Object?>{
        'store_format': 1,
        'snapshot': <String, Object?>{
          'byte_len': utf8.encode(projectJson).length,
          'sha256': _sequence.toRadixString(16).padLeft(64, '0'),
        },
      }),
    );
    _projectsByHead[head.canonicalJson] = projectJson;
    return head;
  }

  @override
  Future<AuthoringRevision3StoreOpenedResult> open({
    required String root,
    required AuthoringAssetVerification verification,
  }) async {
    final rawHead = await File(
      p.join(root, 'gore-project.json'),
    ).readAsString();
    final head = AuthoringWorkingHead.fromCanonicalJson(rawHead);
    final project = _projectsByHead[rawHead];
    if (project == null) throw StateError('unknown revision-3 head');
    return AuthoringRevision3StoreOpenedResult.fromJson(
      _revision3OpenedResponse(head, project),
    );
  }

  @override
  Future<AuthoringRevision3StoreOpenedResult> openHeadBytes({
    required String root,
    required AuthoringWorkingHead head,
    required AuthoringAssetVerification verification,
  }) async {
    final project = _projectsByHead[head.canonicalJson];
    if (project == null) throw StateError('unknown revision-3 checkpoint');
    return AuthoringRevision3StoreOpenedResult.fromJson(
      _revision3OpenedResponse(head, project),
    );
  }

  @override
  Future<AuthoringRevision3CheckpointPreparation> prepareCheckpoint({
    required String root,
    required AuthoringWorkingHead? expectedHead,
    required String projectJson,
  }) async {
    final headFile = File(p.join(root, 'gore-project.json'));
    final actual = await headFile.exists()
        ? await headFile.readAsString()
        : null;
    if (actual != expectedHead?.canonicalJson) {
      throw const ModFfiException(
        command: 'authoring_store_prepare_revision3_checkpoint',
        code: 'AUTHORING_STORE_HEAD_CONFLICT',
        message: 'fake revision-3 head CAS rejected',
      );
    }
    final head = register(projectJson);
    return AuthoringRevision3CheckpointPreparation.fromJson(<String, Object?>{
      'ok': true,
      'head_json': head.canonicalJson,
    });
  }

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
  }) async {
    buildCalls++;
    buildCurrentProjects.add(currentProjectJson);
    buildHeads.add(expectedHead);
    final actual = await File(p.join(root, 'gore-project.json')).readAsString();
    if (actual != expectedHead.canonicalJson ||
        _projectsByHead[actual] != currentProjectJson) {
      throw const ModFfiException(
        command: 'authoring_store_build_revision3_reviewed_dataasset_v1',
        code: 'AUTHORING_REVISION3_DATAASSET_BUILD_HEAD_CONFLICT',
        message: 'fake reviewed DataAsset basis CAS rejected',
      );
    }
    final result = _sessionReviewedDataAssetBuildResult(
      head: expectedHead,
      projectJson: currentProjectJson,
      targetPath: targetPath,
      packName: packName,
      output: output,
    );
    final hook = afterBuild;
    afterBuild = null;
    await hook?.call(root, result);
    return result;
  }

  @override
  dynamic noSuchMethod(Invocation invocation) => throw UnsupportedError(
    'unexpected fake revision-3 Store call: ${invocation.memberName}',
  );
}

Map<String, Object?> _revision3OpenedResponse(
  AuthoringWorkingHead head,
  String projectJson,
) => <String, Object?>{
  'ok': true,
  'head_json': head.canonicalJson,
  'project_json': projectJson,
};

AuthoringRevision3ReviewedDataAssetBuildResult
_sessionReviewedDataAssetBuildResult({
  required AuthoringWorkingHead head,
  required String projectJson,
  required String targetPath,
  required String packName,
  required String output,
}) => AuthoringRevision3ReviewedDataAssetBuildResult.fromJson(
  <String, Object?>{
    'ok': true,
    'outcome': 'built',
    'basis_head_json': head.canonicalJson,
    'project_id': revision3VoiceFixtureProjectId,
    'project_revision': 7,
    'target_path': targetPath,
    'pack_name': packName,
    'output': output,
    'files': <Object?>[
      <String, Object?>{
        'relative_name': '$packName.pak',
        'byte_len': 101,
        'sha256': List<String>.filled(64, 'a').join(),
      },
      <String, Object?>{
        'relative_name': '$packName.ucas',
        'byte_len': 102,
        'sha256': List<String>.filled(64, 'b').join(),
      },
      <String, Object?>{
        'relative_name': '$packName.utoc',
        'byte_len': 103,
        'sha256': List<String>.filled(64, 'c').join(),
      },
    ],
    'receipt': <String, Object?>{
      'format':
          'gore.authoring.managed-revision3-reviewed-dataasset-build-receipt.v1',
      'relative_name': 'gore-authoring-dataasset-build.json',
      'byte_len': 456,
      'sha256': List<String>.filled(64, 'd').join(),
    },
    'build_authority': 'reviewed_fixed_leaf_single_package_triplet',
    'artifact_publication_status': 'published',
    'deployment_status': 'not_performed',
    'runtime_status': 'runtime_unqualified',
    'retry_safe': false,
    'warning': null,
  },
  expectedHead: head,
  expectedProjectJson: projectJson,
  expectedTargetPath: targetPath,
  expectedPackName: packName,
  expectedOutput: output,
);

class _InjectedCrash implements Exception {
  const _InjectedCrash();
}
