import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/managed_project_lock.dart';
import 'package:gore_mod/project/managed_project_session.dart';
import 'package:gore_mod/project/project_atomic_io.dart';
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

  test('create, save, close, and open preserve the exact checkpoint', () async {
    final root = Directory(p.join(fixture.path, 'project'));
    await root.create();
    final store = _FakeManagedStore();
    final firstProject = _projectJson(revision: 0, name: 'First');
    final secondProject = _projectJson(revision: 1, name: 'Second');

    final created = await ManagedAuthoringProjectSession.create(
      root: root,
      store: store,
      projectJson: firstProject,
      profile: AuthoringValidationProfile.experimental,
    );
    final firstHead = created.head.canonicalJson;
    expect(created.profile, AuthoringValidationProfile.experimental);
    expect(created.projectJson, firstProject);
    expect(await created.headFile.readAsString(), firstHead);

    await created.save(secondProject);
    final secondHead = created.head.canonicalJson;
    expect(secondHead, isNot(firstHead));
    expect(created.projectJson, secondProject);
    expect(await created.headFile.readAsString(), secondHead);
    expect(created.requiresReopen, isFalse);
    await created.close();
    await created.close();

    final reopened = await ManagedAuthoringProjectSession.open(
      root: root,
      store: store,
      profile: AuthoringValidationProfile.experimental,
    );
    expect(reopened.profile, AuthoringValidationProfile.experimental);
    expect(reopened.head.canonicalJson, secondHead);
    expect(reopened.projectJson, secondProject);
    await reopened.close();

    expect(store.openVerifications, isNotEmpty);
    expect(
      store.openVerifications,
      everyElement(AuthoringAssetVerification.full),
    );
    expect(store.headVerifications, isNotEmpty);
    expect(
      store.headVerifications,
      everyElement(AuthoringAssetVerification.full),
    );
  });

  test(
    'shared verifyCurrentHead fully reopens without preparing a checkpoint',
    () async {
      final root = Directory(p.join(fixture.path, 'verify_project'));
      await root.create();
      final store = _FakeManagedStore();
      final project = _projectJson(revision: 0, name: 'Verify shared core');
      final session = await ManagedAuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: project,
        profile: AuthoringValidationProfile.production,
      );
      final prepareCalls = store.prepareCalls;
      final openCalls = store.openVerifications.length;
      final headOpenCalls = store.headVerifications.length;
      final exactHead = session.head.canonicalJson;

      await session.verifyCurrentHead();

      expect(store.prepareCalls, prepareCalls);
      expect(store.openVerifications.length, openCalls + 1);
      expect(store.openVerifications.last, AuthoringAssetVerification.full);
      expect(store.headVerifications.length, headOpenCalls);
      expect(session.projectJson, project);
      expect(session.head.canonicalJson, exactHead);
      expect(session.requiresReopen, isFalse);
      await session.close();
    },
  );

  test(
    'production adapter routes only through additive document commands',
    () async {
      final project = _revision2ProjectJson(revision: 1, name: 'Adapter');
      final fixtureStore = _FakeManagedStore();
      final head = fixtureStore.register(project);
      final core = FakeGoreCoreFfiService(
        responses: {
          'authoring_store_open_document': _openedResponse(head, project),
          'authoring_store_prepare_document_checkpoint': _preparedResponse(
            head,
            project,
          ),
          'authoring_store_open_head_bytes_document': _openedResponse(
            head,
            project,
          ),
        },
      );
      final adapter = ModFfiManagedAuthoringStore(ModFfi(core));

      await adapter.open(
        root: fixture.path,
        verification: AuthoringAssetVerification.full,
        profile: AuthoringValidationProfile.production,
      );
      await adapter.prepareCheckpoint(
        root: fixture.path,
        expectedHead: head,
        projectJson: project,
        profile: AuthoringValidationProfile.production,
      );
      await adapter.openHeadBytes(
        root: fixture.path,
        head: head,
        verification: AuthoringAssetVerification.full,
        profile: AuthoringValidationProfile.production,
      );

      expect(core.calls.map((call) => call.command), <String>[
        'authoring_store_open_document',
        'authoring_store_prepare_document_checkpoint',
        'authoring_store_open_head_bytes_document',
      ]);
    },
  );

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

  test('one exact-CAS save advances revision 1 to revision 2', () async {
    final root = Directory(p.join(fixture.path, 'project'));
    await root.create();
    final store = _FakeManagedStore();
    final revision1 = _projectJson(revision: 0, name: 'Before migration');
    final revision2 = _revision2ProjectJson(
      revision: 1,
      name: 'After migration',
    );
    final session = await ManagedAuthoringProjectSession.create(
      root: root,
      store: store,
      projectJson: revision1,
      profile: AuthoringValidationProfile.production,
    );
    expect(session.blocksBuild, isFalse);

    await session.save(revision2);
    final revision2Head = session.head.canonicalJson;
    expect(session.projectJson, revision2);
    expect(session.blocksBuild, isTrue);
    expect(
      session.diagnostics.single.code,
      'REVISION2_COMBINED_VALIDATION_UNAVAILABLE',
    );
    expect(await session.headFile.readAsString(), revision2Head);
    await session.close();

    final reopened = await ManagedAuthoringProjectSession.open(
      root: root,
      store: store,
      profile: AuthoringValidationProfile.production,
    );
    expect(reopened.head.canonicalJson, revision2Head);
    expect(reopened.projectJson, revision2);
    expect(reopened.blocksBuild, isTrue);
    expect(
      store.openVerifications,
      everyElement(AuthoringAssetVerification.full),
    );
    expect(
      store.headVerifications,
      everyElement(AuthoringAssetVerification.full),
    );
    await reopened.close();
  });

  test(
    'derive rejection observes latest state and performs zero writes',
    () async {
      final root = Directory(p.join(fixture.path, 'project'));
      await root.create();
      final store = _FakeManagedStore();
      final original = _projectJson(revision: 0, name: 'Original');
      final session = await ManagedAuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: original,
        profile: AuthoringValidationProfile.experimental,
      );
      final headBefore = await session.headFile.readAsBytes();
      final prepareCallsBefore = store.prepareCalls;
      final headVerificationsBefore = store.headVerifications.length;
      final openVerificationsBefore = store.openVerifications.length;

      final value = await session.deriveAndSave<String>((latestProjectJson) {
        expect(latestProjectJson, original);
        return const ManagedProjectDerivedRejection('rejected');
      });

      expect(value, 'rejected');
      expect(store.prepareCalls, prepareCallsBefore);
      expect(store.headVerifications.length, headVerificationsBefore);
      expect(store.openVerifications.length, openVerificationsBefore);
      expect(await session.headFile.readAsBytes(), headBefore);
      expect(session.projectJson, original);
      expect(session.requiresReopen, isFalse);
      await session.close();
    },
  );

  test(
    'derive candidate publishes through the complete verified save lane',
    () async {
      final root = Directory(p.join(fixture.path, 'project'));
      await root.create();
      final store = _FakeManagedStore();
      final original = _projectJson(revision: 0, name: 'Original');
      final candidate = _projectJson(revision: 1, name: 'Derived');
      final session = await ManagedAuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: original,
        profile: AuthoringValidationProfile.experimental,
      );
      final prepareCallsBefore = store.prepareCalls;
      final headVerificationsBefore = store.headVerifications.length;
      final openVerificationsBefore = store.openVerifications.length;

      final value = await session.deriveAndSave<int>(
        (latestProjectJson) => ManagedProjectDerivedCandidate(
          projectJson: candidate,
          value: latestProjectJson == original ? 42 : -1,
        ),
      );

      expect(value, 42);
      expect(session.projectJson, candidate);
      expect(store.prepareCalls, prepareCallsBefore + 1);
      expect(
        store.headVerifications.length,
        greaterThan(headVerificationsBefore),
      );
      expect(store.openVerifications.length, openVerificationsBefore + 1);
      expect(await session.headFile.readAsString(), session.head.canonicalJson);
      expect(session.requiresReopen, isFalse);
      await session.close();
    },
  );

  test(
    'derive invocations observe prior queued saves in invocation order',
    () async {
      final root = Directory(p.join(fixture.path, 'project'));
      await root.create();
      final store = _FakeManagedStore();
      final original = _projectJson(revision: 0, name: 'Original');
      final first = _projectJson(revision: 1, name: 'First');
      final second = _projectJson(revision: 2, name: 'Second');
      final session = await ManagedAuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: original,
        profile: AuthoringValidationProfile.experimental,
      );
      final observed = <String>[];

      final firstSave = session.save(first);
      final derived = session.deriveAndSave<String>((latestProjectJson) {
        observed.add(latestProjectJson);
        return ManagedProjectDerivedCandidate(
          projectJson: second,
          value: 'published',
        );
      });
      await firstSave;
      expect(await derived, 'published');

      expect(observed, <String>[first]);
      expect(session.projectJson, second);
      await session.close();
    },
  );

  test('derive callback cannot await a save on its own session lane', () async {
    final root = Directory(p.join(fixture.path, 'project'));
    await root.create();
    final session = await ManagedAuthoringProjectSession.create(
      root: root,
      store: _FakeManagedStore(),
      projectJson: _projectJson(revision: 0, name: 'Original'),
      profile: AuthoringValidationProfile.experimental,
    );

    await expectLater(
      session
          .deriveAndSave<void>((_) async {
            await session.save(_projectJson(revision: 1, name: 'Reentrant'));
            return const ManagedProjectDerivedRejection(null);
          })
          .timeout(const Duration(seconds: 1)),
      throwsA(isA<ManagedProjectReentrantOperationException>()),
    );
    expect(session.requiresReopen, isFalse);
    await session.close();
  });

  test('derive callback cannot await a nested derive on its session', () async {
    final root = Directory(p.join(fixture.path, 'project'));
    await root.create();
    final session = await ManagedAuthoringProjectSession.create(
      root: root,
      store: _FakeManagedStore(),
      projectJson: _projectJson(revision: 0, name: 'Original'),
      profile: AuthoringValidationProfile.experimental,
    );

    await expectLater(
      session
          .deriveAndSave<void>((_) async {
            await session.deriveAndSave<void>(
              (_) => const ManagedProjectDerivedRejection(null),
            );
            return const ManagedProjectDerivedRejection(null);
          })
          .timeout(const Duration(seconds: 1)),
      throwsA(isA<ManagedProjectReentrantOperationException>()),
    );
    expect(session.requiresReopen, isFalse);
    await session.close();
  });

  test('derive callback cannot close its own active session lane', () async {
    final root = Directory(p.join(fixture.path, 'project'));
    await root.create();
    final original = _projectJson(revision: 0, name: 'Original');
    final session = await ManagedAuthoringProjectSession.create(
      root: root,
      store: _FakeManagedStore(),
      projectJson: original,
      profile: AuthoringValidationProfile.experimental,
    );

    await expectLater(
      session
          .deriveAndSave<void>((_) async {
            await session.close();
            return const ManagedProjectDerivedRejection(null);
          })
          .timeout(const Duration(seconds: 1)),
      throwsA(isA<ManagedProjectReentrantOperationException>()),
    );
    expect(session.isClosed, isFalse);
    expect(
      await session.deriveAndSave<String>(
        (_) => const ManagedProjectDerivedRejection('still open'),
      ),
      'still open',
    );
    await session.close();
  });

  test(
    'external callers still queue behind an active derive callback',
    () async {
      final root = Directory(p.join(fixture.path, 'project'));
      await root.create();
      final store = _FakeManagedStore();
      final original = _projectJson(revision: 0, name: 'Original');
      final external = _projectJson(revision: 1, name: 'External queued save');
      final session = await ManagedAuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: original,
        profile: AuthoringValidationProfile.experimental,
      );
      final entered = Completer<void>();
      final release = Completer<void>();
      final derived = session.deriveAndSave<String>((latestProjectJson) async {
        expect(latestProjectJson, original);
        entered.complete();
        await release.future;
        return const ManagedProjectDerivedRejection('rejected');
      });
      await entered.future.timeout(const Duration(seconds: 1));

      final queuedSave = session.save(external);
      expect(session.projectJson, original);
      release.complete();
      expect(await derived.timeout(const Duration(seconds: 1)), 'rejected');
      await queuedSave.timeout(const Duration(seconds: 1));
      expect(session.projectJson, external);
      await session.close();
    },
  );

  test('derive failures preserve the same poison semantics as save', () async {
    final root = Directory(p.join(fixture.path, 'project'));
    await root.create();
    final store = _FakeManagedStore();
    final original = _projectJson(revision: 0, name: 'Original');
    final session = await ManagedAuthoringProjectSession.create(
      root: root,
      store: store,
      projectJson: original,
      profile: AuthoringValidationProfile.experimental,
    );

    await expectLater(
      session.deriveAndSave<void>((_) => throw StateError('derive failed')),
      throwsStateError,
    );
    expect(session.requiresReopen, isFalse);
    expect(
      await session.deriveAndSave<String>(
        (_) => const ManagedProjectDerivedRejection('still usable'),
      ),
      'still usable',
    );

    store.nextPrepareError = const ModFfiException(
      command: 'authoring_store_prepare_checkpoint',
      code: 'AUTHORING_STORE_HEAD_CONFLICT',
      message: 'injected native head conflict',
    );
    await expectLater(
      session.deriveAndSave<void>(
        (_) => ManagedProjectDerivedCandidate(
          projectJson: _projectJson(revision: 1, name: 'Candidate'),
          value: null,
        ),
      ),
      throwsA(isA<ManagedProjectHeadConflictException>()),
    );
    expect(session.projectJson, original);
    expect(session.requiresReopen, isTrue);
    await session.close();
  });

  test(
    'derive candidate detects a stale published head before prepare',
    () async {
      final root = Directory(p.join(fixture.path, 'project'));
      await root.create();
      final store = _FakeManagedStore();
      final original = _projectJson(revision: 0, name: 'Original');
      final session = await ManagedAuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: original,
        profile: AuthoringValidationProfile.experimental,
      );
      final prepareCallsBefore = store.prepareCalls;
      final externalProject = _projectJson(revision: 9, name: 'External');
      final externalHead = store.register(externalProject);
      await session.headFile.writeAsString(
        externalHead.canonicalJson,
        flush: true,
      );

      await expectLater(
        session.deriveAndSave<void>((latestProjectJson) {
          expect(latestProjectJson, original);
          return ManagedProjectDerivedCandidate(
            projectJson: _projectJson(revision: 1, name: 'Local'),
            value: null,
          );
        }),
        throwsA(isA<ManagedProjectHeadConflictException>()),
      );

      expect(store.prepareCalls, prepareCallsBefore);
      expect(await session.headFile.readAsString(), externalHead.canonicalJson);
      expect(session.projectJson, original);
      expect(session.requiresReopen, isTrue);
      await session.close();
    },
  );

  test(
    'derive rejection checks missing, noncanonical, and replaced heads first',
    () async {
      for (final mode in <String>['missing', 'noncanonical', 'replaced']) {
        final root = Directory(p.join(fixture.path, 'project-$mode'));
        await root.create();
        final store = _FakeManagedStore();
        final original = _projectJson(revision: 0, name: 'Original');
        final session = await ManagedAuthoringProjectSession.create(
          root: root,
          store: store,
          projectJson: original,
          profile: AuthoringValidationProfile.experimental,
        );
        final prepareCallsBefore = store.prepareCalls;
        final openCallsBefore = store.openVerifications.length;
        final headCallsBefore = store.headVerifications.length;
        switch (mode) {
          case 'missing':
            await session.headFile.delete();
          case 'noncanonical':
            await session.headFile.writeAsString(
              '${session.head.canonicalJson}\n',
              flush: true,
            );
          case 'replaced':
            final externalHead = store.register(
              _projectJson(revision: 9, name: 'External'),
            );
            await session.headFile.writeAsString(
              externalHead.canonicalJson,
              flush: true,
            );
        }
        var callbackInvoked = false;

        await expectLater(
          session.deriveAndSave<void>((_) {
            callbackInvoked = true;
            return const ManagedProjectDerivedRejection(null);
          }),
          throwsA(isA<ManagedProjectSessionException>()),
        );

        expect(callbackInvoked, isFalse, reason: mode);
        expect(store.prepareCalls, prepareCallsBefore, reason: mode);
        expect(store.openVerifications.length, openCallsBefore, reason: mode);
        expect(store.headVerifications.length, headCallsBefore, reason: mode);
        expect(session.requiresReopen, isTrue, reason: mode);
        await session.close();
      }
    },
  );

  test(
    'async rejection fails if the exact head drifts during callback',
    () async {
      final root = Directory(p.join(fixture.path, 'project'));
      await root.create();
      final store = _FakeManagedStore();
      final original = _projectJson(revision: 0, name: 'Original');
      final session = await ManagedAuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: original,
        profile: AuthoringValidationProfile.experimental,
      );
      final prepareCallsBefore = store.prepareCalls;
      final openCallsBefore = store.openVerifications.length;
      final headCallsBefore = store.headVerifications.length;
      final entered = Completer<void>();
      final release = Completer<void>();
      final derived = session.deriveAndSave<void>((_) async {
        entered.complete();
        await release.future;
        return const ManagedProjectDerivedRejection(null);
      });
      await entered.future.timeout(const Duration(seconds: 1));
      final externalHead = store.register(
        _projectJson(revision: 9, name: 'External'),
      );
      await session.headFile.writeAsString(
        externalHead.canonicalJson,
        flush: true,
      );
      release.complete();

      await expectLater(
        derived.timeout(const Duration(seconds: 1)),
        throwsA(isA<ManagedProjectHeadConflictException>()),
      );
      expect(store.prepareCalls, prepareCallsBefore);
      expect(store.openVerifications.length, openCallsBefore);
      expect(store.headVerifications.length, headCallsBefore);
      expect(await session.headFile.readAsString(), externalHead.canonicalJson);
      expect(session.projectJson, original);
      expect(session.requiresReopen, isTrue);
      await session.close();
    },
  );

  test(
    'async callback failure still poisons if the exact head drifts',
    () async {
      final root = Directory(p.join(fixture.path, 'project'));
      await root.create();
      final store = _FakeManagedStore();
      final original = _projectJson(revision: 0, name: 'Original');
      final session = await ManagedAuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: original,
        profile: AuthoringValidationProfile.experimental,
      );
      final prepareCallsBefore = store.prepareCalls;
      final entered = Completer<void>();
      final release = Completer<void>();
      final derived = session.deriveAndSave<void>((_) async {
        entered.complete();
        await release.future;
        throw StateError('callback failed');
      });
      await entered.future.timeout(const Duration(seconds: 1));
      final externalHead = store.register(
        _projectJson(revision: 9, name: 'External'),
      );
      await session.headFile.writeAsString(
        externalHead.canonicalJson,
        flush: true,
      );
      release.complete();

      await expectLater(
        derived.timeout(const Duration(seconds: 1)),
        throwsA(isA<ManagedProjectHeadConflictException>()),
      );
      expect(store.prepareCalls, prepareCallsBefore);
      expect(await session.headFile.readAsString(), externalHead.canonicalJson);
      expect(session.projectJson, original);
      expect(session.requiresReopen, isTrue);
      await session.close();
    },
  );

  test('save rejects exact head drift without overwriting it', () async {
    final root = Directory(p.join(fixture.path, 'project'));
    await root.create();
    final store = _FakeManagedStore();
    final original = _projectJson(revision: 0, name: 'Original');
    final session = await ManagedAuthoringProjectSession.create(
      root: root,
      store: store,
      projectJson: original,
      profile: AuthoringValidationProfile.experimental,
    );
    final sessionHead = session.head.canonicalJson;
    final externalProject = _projectJson(revision: 8, name: 'External');
    final externalHead = store.register(externalProject);
    store.afterPrepare = (root, _, _) async {
      await File(
        p.join(root, 'gore-project.json'),
      ).writeAsString(externalHead.canonicalJson, flush: true);
    };

    await expectLater(
      session.save(_projectJson(revision: 1, name: 'Local')),
      throwsA(isA<ManagedProjectHeadConflictException>()),
    );
    expect(await session.headFile.readAsString(), externalHead.canonicalJson);
    expect(session.head.canonicalJson, sessionHead);
    expect(session.projectJson, original);
    expect(session.requiresReopen, isTrue);
    await expectLater(
      session.save(_projectJson(revision: 2, name: 'Another local edit')),
      throwsA(isA<ManagedProjectVerificationException>()),
    );
    await session.close();
  });

  test('native prepare CAS conflicts also require a reopen', () async {
    final root = Directory(p.join(fixture.path, 'project'));
    await root.create();
    final store = _FakeManagedStore();
    final session = await ManagedAuthoringProjectSession.create(
      root: root,
      store: store,
      projectJson: _projectJson(revision: 0, name: 'Original'),
      profile: AuthoringValidationProfile.experimental,
    );
    final exactHead = session.head.canonicalJson;
    store.nextPrepareError = const ModFfiException(
      command: 'authoring_store_prepare_checkpoint',
      code: 'AUTHORING_STORE_HEAD_CONFLICT',
      message: 'injected native head conflict',
    );

    await expectLater(
      session.save(_projectJson(revision: 1, name: 'Local')),
      throwsA(isA<ManagedProjectHeadConflictException>()),
    );
    expect(session.requiresReopen, isTrue);
    expect(await session.headFile.readAsString(), exactHead);
    await session.close();
  });

  test('noncanonical disk head poisons the session before prepare', () async {
    final root = Directory(p.join(fixture.path, 'project'));
    await root.create();
    final store = _FakeManagedStore();
    final session = await ManagedAuthoringProjectSession.create(
      root: root,
      store: store,
      projectJson: _projectJson(revision: 0, name: 'Original'),
      profile: AuthoringValidationProfile.experimental,
    );
    final prepareCallsAfterCreate = store.prepareCalls;
    await session.headFile.writeAsString(
      '${session.head.canonicalJson}\n',
      flush: true,
    );

    await expectLater(
      session.save(_projectJson(revision: 1, name: 'Rejected')),
      throwsA(isA<ManagedProjectVerificationException>()),
    );
    expect(store.prepareCalls, prepareCallsAfterCreate);
    expect(session.requiresReopen, isTrue);
    await session.close();
  });

  test('native corrupt-head errors poison the session', () async {
    final root = Directory(p.join(fixture.path, 'project'));
    await root.create();
    final store = _FakeManagedStore();
    final session = await ManagedAuthoringProjectSession.create(
      root: root,
      store: store,
      projectJson: _projectJson(revision: 0, name: 'Original'),
      profile: AuthoringValidationProfile.experimental,
    );
    store.nextPrepareError = const ModFfiException(
      command: 'authoring_store_prepare_checkpoint',
      code: 'AUTHORING_STORE_JSON_NONCANONICAL',
      message: 'injected corrupt fixed head',
    );

    await expectLater(
      session.save(_projectJson(revision: 1, name: 'Rejected')),
      throwsA(isA<ManagedProjectVerificationException>()),
    );
    expect(session.requiresReopen, isTrue);
    await session.close();
  });

  test(
    'open repairs a fully verified head after an interrupted publish',
    () async {
      final root = Directory(p.join(fixture.path, 'project'));
      await root.create();
      final store = _FakeManagedStore();
      final project = _projectJson(revision: 0, name: 'Crash recovery');
      final replacement = AtomicByteReplacement(
        operationIdFactory: () => '70000000000000000000000000000001',
        onPhase: (phase) {
          if (phase == AtomicSwapPhase.tempPromoted) {
            throw const _InjectedCrash();
          }
        },
      );

      await expectLater(
        ManagedAuthoringProjectSession.create(
          root: root,
          store: store,
          projectJson: project,
          profile: AuthoringValidationProfile.experimental,
          replacement: replacement,
        ),
        throwsA(isA<_InjectedCrash>()),
      );
      final headFile = File(p.join(root.path, 'gore-project.json'));
      final journal = File(AtomicByteReplacement.journalPathFor(headFile));
      expect(await headFile.exists(), isTrue);
      expect(await journal.exists(), isTrue);

      final recovered = await ManagedAuthoringProjectSession.open(
        root: root,
        store: store,
        profile: AuthoringValidationProfile.experimental,
      );
      expect(recovered.projectJson, project);
      expect(await journal.exists(), isFalse);
      expect(store.headVerifications, isNotEmpty);
      await recovered.close();
    },
  );

  test(
    'a failed full post-publication reopen poisons only the session',
    () async {
      final root = Directory(p.join(fixture.path, 'project'));
      await root.create();
      final store = _FakeManagedStore();
      final original = _projectJson(revision: 0, name: 'Original');
      final saved = _projectJson(revision: 1, name: 'Saved');
      final session = await ManagedAuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: original,
        profile: AuthoringValidationProfile.experimental,
      );
      store.nextOpenProjectOverride = _projectJson(
        revision: 99,
        name: 'Injected mismatch',
      );

      await expectLater(
        session.save(saved),
        throwsA(isA<ManagedProjectVerificationException>()),
      );
      expect(session.projectJson, original);
      expect(session.requiresReopen, isTrue);
      await session.close();

      final reopened = await ManagedAuthoringProjectSession.open(
        root: root,
        store: store,
        profile: AuthoringValidationProfile.experimental,
      );
      expect(reopened.projectJson, saved);
      await reopened.close();
    },
  );

  test('open failure releases the session lock', () async {
    final root = Directory(p.join(fixture.path, 'project'));
    await root.create();
    final store = _FakeManagedStore();
    final created = await ManagedAuthoringProjectSession.create(
      root: root,
      store: store,
      projectJson: _projectJson(revision: 0, name: 'Lock release'),
      profile: AuthoringValidationProfile.production,
    );
    await created.close();
    store.failNextOpen = true;

    await expectLater(
      ManagedAuthoringProjectSession.open(
        root: root,
        store: store,
        profile: AuthoringValidationProfile.production,
      ),
      throwsA(isA<StateError>()),
    );

    final lock = await ManagedProjectSessionLock.acquire(root);
    await lock.release();
  });

  test('create never replaces an existing managed head', () async {
    final root = Directory(p.join(fixture.path, 'project'));
    await root.create();
    final store = _FakeManagedStore();
    final first = await ManagedAuthoringProjectSession.create(
      root: root,
      store: store,
      projectJson: _projectJson(revision: 0, name: 'Existing'),
      profile: AuthoringValidationProfile.experimental,
    );
    final exactHead = await first.headFile.readAsString();
    await first.close();

    await expectLater(
      ManagedAuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: _projectJson(revision: 1, name: 'Replacement'),
        profile: AuthoringValidationProfile.experimental,
      ),
      throwsA(isA<ManagedProjectAlreadyInitializedException>()),
    );
    expect(
      await File(p.join(root.path, 'gore-project.json')).readAsString(),
      exactHead,
    );

    final lock = await ManagedProjectSessionLock.acquire(root);
    await lock.release();
  });

  test('create leaves an interrupted existing save untouched', () async {
    final root = Directory(p.join(fixture.path, 'project'));
    await root.create();
    final store = _FakeManagedStore();
    final original = _projectJson(revision: 0, name: 'Existing');
    final created = await ManagedAuthoringProjectSession.create(
      root: root,
      store: store,
      projectJson: original,
      profile: AuthoringValidationProfile.experimental,
    );
    final originalHead = created.head;
    await created.close();

    final pendingProject = _projectJson(revision: 1, name: 'Pending save');
    final pendingHead = store.register(pendingProject);
    final replacement = AtomicByteReplacement(
      operationIdFactory: () => '72000000000000000000000000000001',
      onPhase: (phase) {
        if (phase == AtomicSwapPhase.tempValidated) {
          throw const _InjectedCrash();
        }
      },
    );
    Future<bool> validate(File candidate) async {
      try {
        final head = AuthoringWorkingHead.fromCanonicalJson(
          await candidate.readAsString(),
        );
        await store.openHeadBytes(
          root: root.path,
          head: head,
          verification: AuthoringAssetVerification.full,
          profile: AuthoringValidationProfile.experimental,
        );
        return true;
      } catch (_) {
        return false;
      }
    }

    await expectLater(
      replacement.replaceIfUnchanged(
        target: created.headFile,
        bytes: utf8.encode(pendingHead.canonicalJson),
        expectedBytes: utf8.encode(originalHead.canonicalJson),
        validate: validate,
      ),
      throwsA(isA<_InjectedCrash>()),
    );
    final journal = File(
      AtomicByteReplacement.journalPathFor(created.headFile),
    );
    expect(await created.headFile.readAsString(), originalHead.canonicalJson);
    expect(await journal.exists(), isTrue);

    await expectLater(
      ManagedAuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: _projectJson(revision: 2, name: 'Must not replace'),
        profile: AuthoringValidationProfile.experimental,
      ),
      throwsA(isA<ManagedProjectAlreadyInitializedException>()),
    );
    expect(await created.headFile.readAsString(), originalHead.canonicalJson);
    expect(await journal.exists(), isTrue);

    final recovered = await ManagedAuthoringProjectSession.open(
      root: root,
      store: store,
      profile: AuthoringValidationProfile.experimental,
    );
    expect(recovered.head.canonicalJson, pendingHead.canonicalJson);
    expect(recovered.projectJson, pendingProject);
    await recovered.close();
  });

  test('create rejects a missing root without creating artifacts', () async {
    final root = Directory(p.join(fixture.path, 'missing-project'));

    await expectLater(
      ManagedAuthoringProjectSession.create(
        root: root,
        store: _FakeManagedStore(),
        projectJson: _projectJson(revision: 0, name: 'Missing'),
        profile: AuthoringValidationProfile.experimental,
      ),
      throwsA(isA<ManagedProjectLockException>()),
    );
    expect(await root.exists(), isFalse);
  });
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

typedef _AfterPrepare =
    FutureOr<void> Function(
      String root,
      AuthoringWorkingHead head,
      String projectJson,
    );

class _FakeManagedStore implements ManagedAuthoringStore {
  final Map<String, String> _projectsByHead = <String, String>{};
  int _sequence = 0;

  _AfterPrepare? afterPrepare;
  String? nextOpenProjectOverride;
  bool failNextOpen = false;
  ModFfiException? nextPrepareError;
  int prepareCalls = 0;
  final List<AuthoringAssetVerification> openVerifications = [];
  final List<AuthoringAssetVerification> headVerifications = [];

  AuthoringWorkingHead register(String projectJson) {
    _sequence++;
    final sha = _sequence.toRadixString(16).padLeft(64, '0');
    final raw = jsonEncode({
      'store_format': 1,
      'snapshot': {'byte_len': utf8.encode(projectJson).length, 'sha256': sha},
    });
    final head = AuthoringWorkingHead.fromCanonicalJson(raw);
    _projectsByHead[head.canonicalJson] = projectJson;
    return head;
  }

  @override
  Future<AuthoringStoreOpenedResult> open({
    required String root,
    required AuthoringAssetVerification verification,
    required AuthoringValidationProfile profile,
  }) async {
    openVerifications.add(verification);
    if (failNextOpen) {
      failNextOpen = false;
      throw StateError('injected open failure');
    }
    final rawHead = await File(
      p.join(root, 'gore-project.json'),
    ).readAsString();
    final head = AuthoringWorkingHead.fromCanonicalJson(rawHead);
    final project = _projectsByHead[rawHead];
    if (project == null) throw StateError('unknown published head');
    final override = nextOpenProjectOverride;
    nextOpenProjectOverride = null;
    return _opened(head, override ?? project);
  }

  @override
  Future<AuthoringStoreOpenedResult> openHeadBytes({
    required String root,
    required AuthoringWorkingHead head,
    required AuthoringAssetVerification verification,
    required AuthoringValidationProfile profile,
  }) async {
    headVerifications.add(verification);
    final project = _projectsByHead[head.canonicalJson];
    if (project == null) throw StateError('unknown checkpoint head');
    return _opened(head, project);
  }

  @override
  Future<AuthoringCheckpointPreparation> prepareCheckpoint({
    required String root,
    required AuthoringWorkingHead? expectedHead,
    required String projectJson,
    required AuthoringValidationProfile profile,
  }) async {
    prepareCalls++;
    final injectedError = nextPrepareError;
    nextPrepareError = null;
    if (injectedError != null) throw injectedError;
    final headFile = File(p.join(root, 'gore-project.json'));
    final actual = await headFile.exists()
        ? await headFile.readAsString()
        : null;
    if (actual != expectedHead?.canonicalJson) {
      throw const ModFfiException(
        command: 'authoring_store_prepare_checkpoint',
        code: 'AUTHORING_STORE_HEAD_CONFLICT',
        message: 'fake native head CAS rejected',
      );
    }
    final head = register(projectJson);
    await afterPrepare?.call(root, head, projectJson);
    afterPrepare = null;
    return AuthoringCheckpointPreparation.fromJson(
      _preparedResponse(head, projectJson),
    );
  }

  AuthoringStoreOpenedResult _opened(
    AuthoringWorkingHead head,
    String projectJson,
  ) => AuthoringStoreOpenedResult.fromJson(_openedResponse(head, projectJson));
}

class _InjectedCrash implements Exception {
  const _InjectedCrash();
}

String _projectJson({required int revision, required String name}) =>
    jsonEncode(<String, Object?>{
      'format': 2,
      'schema_revision': 1,
      'project_id': '00000000000000000000000000000001',
      'revision': revision,
      'meta': <String, Object?>{
        'name': name,
        'version': '1.0.0',
        'author': 'session tests',
      },
      'target': <String, Object?>{
        'executable': <String, Object?>{
          'byte_len': 1,
          'sha256': List.filled(64, '4').join(),
        },
      },
      'authoring_locales': <Object?>[],
      'entities': <String, Object?>{},
      'asset_store': <String, Object?>{'assets': <String, Object?>{}},
    });

String _revision2ProjectJson({required int revision, required String name}) =>
    jsonEncode(<String, Object?>{
      'format': 2,
      'schema_revision': 2,
      'project_id': '00000000000000000000000000000002',
      'revision': revision,
      'meta': <String, Object?>{
        'name': name,
        'version': '1.0.0',
        'author': 'session tests',
      },
      'target': <String, Object?>{
        'executable': <String, Object?>{
          'byte_len': 1,
          'sha256': List.filled(64, '5').join(),
        },
      },
      'authoring_locales': <Object?>[],
      'entities': <String, Object?>{},
      'asset_store': <String, Object?>{'assets': <String, Object?>{}},
    });

Map<String, Object?>
_revision2CombinedValidationDiagnostic() => <String, Object?>{
  'code': 'REVISION2_COMBINED_VALIDATION_UNAVAILABLE',
  'severity': 'error',
  'entity': null,
  'property_path': 'schema_revision',
  'message':
      'schema revision 2 is not build-ready until combined story, voice, localization, and asset validation is implemented',
  'related_entities': <Object?>[],
  'blocks_build': true,
};

bool _isRevision2(String projectJson) =>
    (jsonDecode(projectJson) as Map<String, Object?>)['schema_revision'] == 2;

Map<String, Object?> _preparedResponse(
  AuthoringWorkingHead head,
  String projectJson,
) {
  final revision2 = _isRevision2(projectJson);
  return <String, Object?>{
    'ok': true,
    'head_json': head.canonicalJson,
    'diagnostics': <Object?>[
      if (revision2) _revision2CombinedValidationDiagnostic(),
    ],
    'blocks_build': revision2,
  };
}

Map<String, Object?> _openedResponse(
  AuthoringWorkingHead head,
  String projectJson,
) => <String, Object?>{
  ..._preparedResponse(head, projectJson),
  'project_json': projectJson,
};
