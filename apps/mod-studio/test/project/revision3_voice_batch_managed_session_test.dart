import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/managed_project_session.dart';

import '_revision3_voice_batch_test_support.dart';

void main() {
  late Directory root;
  ManagedRevision3AuthoringProjectSession? session;

  setUp(() async {
    root = await Directory.systemTemp.createTemp('gore_voice_batch_session_');
  });

  tearDown(() async {
    await session?.close();
    if (await root.exists()) await root.delete(recursive: true);
  });

  Future<
    (
      Revision3VoiceBatchTestFixture,
      VoiceBatchManagedStore,
      ManagedRevision3AuthoringProjectSession,
    )
  >
  createSession() async {
    final fixture = Revision3VoiceBatchTestFixture.create();
    final store = VoiceBatchManagedStore(fixture);
    final created = await ManagedRevision3AuthoringProjectSession.create(
      root: root,
      store: store,
      projectJson: fixture.projectJson,
    );
    session = created;
    return (fixture, store, created);
  }

  test(
    'session plans against only its exact current head and project bytes',
    () async {
      final (fixture, store, created) = await createSession();

      final plan = await created.planVoiceBatchV1(
        gameRoot: voiceBatchTestGameRoot,
        sourceFolder: voiceBatchTestSourceFolder,
        locale: voiceBatchTestLocale,
      );

      expect(plan.basisHead.canonicalJson, fixture.basisHead.canonicalJson);
      expect(store.planCalls, 1);
      expect(store.receivedPlanRoot, created.root.path);
      expect(store.receivedPlanGameRoot, voiceBatchTestGameRoot);
      expect(store.receivedPlanSourceFolder, voiceBatchTestSourceFolder);
      expect(store.receivedPlanLocale, voiceBatchTestLocale);
      expect(store.receivedPlanProjectJson, fixture.projectJson);
      expect(
        store.receivedPlanHead?.canonicalJson,
        fixture.basisHead.canonicalJson,
      );
      expect(created.projectRevision, 7);
      expect(created.requiresReopen, isFalse);
    },
  );

  test(
    'stale reviewed plan performs zero native prepare and zero publication',
    () async {
      final (fixture, store, created) = await createSession();
      final staleFixture = Revision3VoiceBatchTestFixture.create(
        projectJson: fixture.projectJson,
        basisHead: voiceBatchDifferentHead(),
      );

      await expectLater(
        created.prepareAndPublishVoiceBatchV1(
          gameRoot: voiceBatchTestGameRoot,
          sourceFolder: voiceBatchTestSourceFolder,
          plan: staleFixture.plan(),
        ),
        throwsA(isA<Revision3VoiceBatchStaleCheckpointException>()),
      );

      expect(store.prepareBatchCalls, 0);
      expect(created.projectRevision, 7);
      expect(created.head.canonicalJson, fixture.basisHead.canonicalJson);
      expect(
        await created.headFile.readAsString(),
        fixture.basisHead.canonicalJson,
      );
      expect(created.requiresReopen, isFalse);
    },
  );

  test(
    'ready folder publishes all recordings as exactly one revision',
    () async {
      final (fixture, store, created) = await createSession();

      final checkpoint = await created.prepareAndPublishVoiceBatchV1(
        gameRoot: voiceBatchTestGameRoot,
        sourceFolder: voiceBatchTestSourceFolder,
        plan: fixture.plan(),
      );

      expect(store.prepareBatchCalls, 1);
      expect(checkpoint.projectRevision, 8);
      expect(checkpoint.importedCount, 1);
      expect(checkpoint.alreadyPresentCount, 0);
      expect(checkpoint.items, hasLength(1));
      expect(created.projectRevision, 8);
      expect(created.projectJson, fixture.candidateProjectJson);
      expect(created.head.canonicalJson, fixture.candidateHead.canonicalJson);
      expect(
        await created.headFile.readAsString(),
        fixture.candidateHead.canonicalJson,
      );
      expect(created.requiresReopen, isFalse);
    },
  );

  test('uncertain native prepare poisons the lease before any retry', () async {
    final (fixture, store, created) = await createSession();
    store.throwUncertainPrepare = true;

    await expectLater(
      created.prepareAndPublishVoiceBatchV1(
        gameRoot: voiceBatchTestGameRoot,
        sourceFolder: voiceBatchTestSourceFolder,
        plan: fixture.plan(),
      ),
      throwsA(isA<ManagedProjectVerificationException>()),
    );

    expect(store.prepareBatchCalls, 1);
    expect(created.projectRevision, 7);
    expect(created.requiresReopen, isTrue);
    await expectLater(
      created.planVoiceBatchV1(
        gameRoot: voiceBatchTestGameRoot,
        sourceFolder: voiceBatchTestSourceFolder,
        locale: voiceBatchTestLocale,
      ),
      throwsA(isA<ManagedProjectVerificationException>()),
    );
    expect(store.planCalls, 0);
    expect(store.prepareBatchCalls, 1);
  });
}
