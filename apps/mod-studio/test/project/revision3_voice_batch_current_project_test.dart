import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/current_project_controller.dart';
import 'package:gore_mod/project/managed_project_session.dart';

import '../support/revision3_voice_fixture.dart';
import '_revision3_voice_batch_test_support.dart';

void main() {
  test(
    'coordinator rejects stale root, project, revision, and head before scan',
    () async {
      final fixture = Revision3VoiceBatchTestFixture.create();
      final lease = _VoiceBatchLease(fixture);
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => lease,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(lease.root);

      final cases =
          <
            ({
              String root,
              String projectId,
              int revision,
              AuthoringWorkingHead head,
            })
          >[
            (
              root: '${lease.root.path}-other',
              projectId: lease.projectId,
              revision: lease.projectRevision,
              head: lease.head,
            ),
            (
              root: lease.root.path,
              projectId: 'ffffffffffffffffffffffffffffffff',
              revision: lease.projectRevision,
              head: lease.head,
            ),
            (
              root: lease.root.path,
              projectId: lease.projectId,
              revision: lease.projectRevision + 1,
              head: lease.head,
            ),
            (
              root: lease.root.path,
              projectId: lease.projectId,
              revision: lease.projectRevision,
              head: voiceBatchDifferentHead(),
            ),
          ];

      for (final testCase in cases) {
        await expectLater(
          coordinator.planCurrentRevision3VoiceBatchV1(
            expectedRoot: testCase.root,
            expectedProjectId: testCase.projectId,
            expectedProjectRevision: testCase.revision,
            expectedHead: testCase.head,
            gameRoot: voiceBatchTestGameRoot,
            sourceFolder: voiceBatchTestSourceFolder,
            locale: voiceBatchTestLocale,
          ),
          throwsA(isA<Revision3VoiceBatchStaleCheckpointException>()),
        );
      }

      expect(lease.planCalls, 0);
      expect(lease.markUncertainCalls, 0);
      expect(lease.requiresReopen, isFalse);
    },
  );

  test('coordinator poisons late plan identity drift', () async {
    final fixture = Revision3VoiceBatchTestFixture.create();
    final lease = _VoiceBatchLease(fixture);
    // Assign after construction so the callback can mutate the exact adopted
    // lease while the native-equivalent read is suspended.
    lease.onPlan = () async {
      lease.projectRevisionValue++;
      return fixture.plan();
    };
    final coordinator = CurrentProjectCoordinator(
      openManagedRevision3: (_) async => lease,
    );
    addTearDown(() async {
      await coordinator.shutdown();
      coordinator.dispose();
    });
    await coordinator.openManagedRevision3(lease.root);

    await expectLater(
      coordinator.planCurrentRevision3VoiceBatchV1(
        expectedRoot: lease.root.path,
        expectedProjectId: revision3VoiceFixtureProjectId,
        expectedProjectRevision: 7,
        expectedHead: fixture.basisHead,
        gameRoot: voiceBatchTestGameRoot,
        sourceFolder: voiceBatchTestSourceFolder,
        locale: voiceBatchTestLocale,
      ),
      throwsA(isA<Revision3VoiceBatchRequiresReopenException>()),
    );

    expect(lease.planCalls, 1);
    expect(lease.markUncertainCalls, 1);
    expect(lease.requiresReopen, isTrue);
    expect(
      (coordinator.state as ManagedRevision3CurrentProjectState).requiresReopen,
      isTrue,
    );
  });

  test(
    'coordinator poisons a publication receipt not adopted by its lease',
    () async {
      final fixture = Revision3VoiceBatchTestFixture.create();
      final checkpoint = await _createPublishedBatchCheckpoint(fixture);
      final lease = _VoiceBatchLease(fixture)
        ..onPrepare = (_) async {
          // The receipt claims publication, but the owning lease does not expose
          // that head/revision. The coordinator must revoke its authority.
          return checkpoint;
        };
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => lease,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(lease.root);

      await expectLater(
        coordinator.importCurrentRevision3VoiceBatchV1(
          expectedRoot: lease.root.path,
          expectedProjectId: lease.projectId,
          expectedProjectRevision: lease.projectRevision,
          expectedHead: lease.head,
          gameRoot: voiceBatchTestGameRoot,
          sourceFolder: voiceBatchTestSourceFolder,
          plan: fixture.plan(),
        ),
        throwsA(isA<Revision3VoiceBatchRequiresReopenException>()),
      );

      expect(lease.prepareCalls, 1);
      expect(lease.markUncertainCalls, 1);
      expect(lease.requiresReopen, isTrue);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isTrue,
      );
    },
  );
}

Future<ManagedRevision3VoiceBatchCheckpoint> _createPublishedBatchCheckpoint(
  Revision3VoiceBatchTestFixture fixture,
) async {
  final root = await Directory.systemTemp.createTemp(
    'gore_voice_batch_receipt_',
  );
  final session = await ManagedRevision3AuthoringProjectSession.create(
    root: root,
    store: VoiceBatchManagedStore(fixture),
    projectJson: fixture.projectJson,
  );
  try {
    return await session.prepareAndPublishVoiceBatchV1(
      gameRoot: voiceBatchTestGameRoot,
      sourceFolder: voiceBatchTestSourceFolder,
      plan: fixture.plan(),
    );
  } finally {
    await session.close();
    if (await root.exists()) await root.delete(recursive: true);
  }
}

final class _VoiceBatchLease
    implements
        ManagedRevision3CurrentProjectLease,
        ManagedRevision3VoiceBatchLease {
  _VoiceBatchLease(this.fixture)
    : root = Directory('voice-batch-owned-root'),
      projectRevisionValue = 7,
      headValue = fixture.basisHead;

  final Revision3VoiceBatchTestFixture fixture;

  @override
  final Directory root;

  int projectRevisionValue;
  AuthoringWorkingHead headValue;
  bool requiresReopenValue = false;
  int planCalls = 0;
  int prepareCalls = 0;
  int markUncertainCalls = 0;
  int closeCalls = 0;
  Future<AuthoringRevision3VoiceBatchPlanResult> Function()? onPlan;
  Future<ManagedRevision3VoiceBatchCheckpoint> Function(
    AuthoringRevision3VoiceBatchPlanResult plan,
  )?
  onPrepare;

  @override
  String get projectId => revision3VoiceFixtureProjectId;

  @override
  int get projectRevision => projectRevisionValue;

  @override
  String get canonicalProjectJson => fixture.projectJson;

  @override
  AuthoringWorkingHead get head => headValue;

  @override
  bool get requiresReopen => requiresReopenValue;

  @override
  bool get supportsVoiceBatch => true;

  @override
  void markRequiresReopenAfterVoiceBatchUncertainty() {
    markUncertainCalls++;
    requiresReopenValue = true;
  }

  @override
  Future<AuthoringRevision3VoiceBatchPlanResult> planVoiceBatchV1({
    required String gameRoot,
    required String sourceFolder,
    required String locale,
  }) {
    planCalls++;
    return onPlan?.call() ?? Future.value(fixture.plan());
  }

  @override
  Future<ManagedRevision3VoiceBatchCheckpoint> prepareAndPublishVoiceBatchV1({
    required String gameRoot,
    required String sourceFolder,
    required AuthoringRevision3VoiceBatchPlanResult plan,
  }) {
    prepareCalls++;
    final callback = onPrepare;
    if (callback == null) {
      return Future.error(StateError('unexpected fake batch prepare'));
    }
    return callback(plan);
  }

  @override
  Future<void> close() async {
    closeCalls++;
  }

  @override
  dynamic noSuchMethod(Invocation invocation) => throw UnsupportedError(
    'unused managed-lease member: ${invocation.memberName}',
  );
}
