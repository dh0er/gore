import 'dart:async';
import 'dart:collection';
import 'dart:convert';
import 'dart:io';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/managed_project_lock.dart';
import 'package:gore_mod/project/managed_project_session.dart';
import 'package:gore_mod/project/project_atomic_io.dart';
import 'package:path/path.dart' as p;

import '../support/revision3_quest_fixture.dart';

void main() {
  late Directory fixture;

  setUp(() async {
    fixture = await Directory.systemTemp.createTemp(
      'gore_managed_revision3_session_',
    );
  });

  tearDown(() async {
    if (await fixture.exists()) await fixture.delete(recursive: true);
  });

  test(
    'production adapter uses only the dedicated revision-3 commands',
    () async {
      final project = _projectJson(revision: 7, name: 'Adapter');
      final fixtureStore = _FakeRevision3Store();
      final head = fixtureStore.register(project);
      final questRequest = AuthoringRevision3QuestDraftRequestV3(
        expectedHead: head,
        expectedProjectId: '00000000000000000000000000000003',
        expectedRevision: 7,
        questId: '00000000000000000000000000000071',
        scriptModuleId: '00000000000000000000000000000072',
        displayName: 'Managed Quest 1',
        intent: _questIntent(1),
      );
      final projectMap = jsonDecode(project) as Map<String, Object?>;
      final questInput = _questInput(
        request: questRequest,
        basisHead: head,
        target: (projectMap['target'] as Map).cast<String, Object?>(),
      );
      final candidateMap = jsonDecode(project) as Map<String, Object?>
        ..['revision'] = 8
        ..['entities'] = <String, Object?>{
          questRequest.questId: _questEntity(
            projectId: questRequest.expectedProjectId,
            request: questRequest,
            input: questInput,
          ),
          questRequest.scriptModuleId: _questModuleEntity(
            projectId: questRequest.expectedProjectId,
            request: questRequest,
            input: questInput,
          ),
        }
        ..['asset_store'] = <String, Object?>{
          'assets': <String, Object?>{
            _questArtifactSha: <String, Object?>{
              'byte_len': 123,
              'media_type':
                  'application/vnd.gore.quest-collision-capability+json;version=2',
            },
          },
        };
      final candidateProject = jsonEncode(candidateMap);
      final candidateHead = fixtureStore.register(candidateProject);
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          'authoring_store_open_revision3': _openedResponse(head, project),
          'authoring_store_prepare_revision3_checkpoint': _preparedResponse(
            head,
          ),
          'authoring_store_open_revision3_head_bytes': _openedResponse(
            head,
            project,
          ),
          'authoring_store_read_revision3_content_index_v1': _contentResponse(
            head,
            project,
          ),
          'authoring_store_prepare_revision3_quest_draft_v3':
              _questPreparedResponse(
                basisHead: head,
                candidateHead: candidateHead,
                candidateProjectJson: candidateProject,
                revision: 8,
                questId: questRequest.questId,
                scriptModuleId: questRequest.scriptModuleId,
              ),
        },
      );
      final adapter = ModFfiManagedRevision3AuthoringStore(ModFfi(core));

      await adapter.open(
        root: fixture.path,
        verification: AuthoringAssetVerification.full,
      );
      await adapter.prepareCheckpoint(
        root: fixture.path,
        expectedHead: head,
        projectJson: project,
      );
      await adapter.openHeadBytes(
        root: fixture.path,
        head: head,
        verification: AuthoringAssetVerification.full,
      );
      await adapter.readContentIndex(root: fixture.path, expectedHead: head);
      await adapter.prepareQuestDraftV3(
        root: fixture.path,
        gameRoot: r'D:\Games\Gothic Remake',
        currentProjectJson: project,
        questRequestJson: questRequest.canonicalJson,
      );

      expect(core.calls.map((call) => call.command), <String>[
        'authoring_store_open_revision3',
        'authoring_store_prepare_revision3_checkpoint',
        'authoring_store_open_revision3_head_bytes',
        'authoring_store_read_revision3_content_index_v1',
        'authoring_store_prepare_revision3_quest_draft_v3',
      ]);
      expect(core.calls[0].payload, <String, Object?>{
        'root': fixture.path,
        'verification': 'full',
      });
      expect(core.calls[1].payload, <String, Object?>{
        'root': fixture.path,
        'expected_head_json': head.canonicalJson,
        'project_json': project,
      });
      expect(core.calls[2].payload, <String, Object?>{
        'root': fixture.path,
        'head_json': head.canonicalJson,
        'verification': 'full',
      });
      expect(core.calls[3].payload, <String, Object?>{
        'expected_head_json': head.canonicalJson,
        'root': fixture.path,
      });
      expect(core.calls[4].payload, <String, Object?>{
        'current_project_json': project,
        'game_root': r'D:\Games\Gothic Remake',
        'quest_request_json': questRequest.canonicalJson,
        'root': fixture.path,
      });
    },
  );

  test(
    'create, save, derive, close, and open preserve exact R3 bytes',
    () async {
      final root = await _projectRoot(fixture);
      final store = _FakeRevision3Store();
      final original = _projectJson(revision: 0, name: 'Original');
      final saved = _projectJson(revision: 1, name: 'Saved');
      final derived = _projectJson(revision: 2, name: 'Derived');

      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: original,
      );
      expect(session.projectJson, original);
      expect(session.projectId, '00000000000000000000000000000003');
      expect(session.projectRevision, 0);
      expect(store.expectedHeads, <String?>[null]);
      expect(await session.headFile.readAsString(), session.head.canonicalJson);

      final firstHead = session.head.canonicalJson;
      await session.save(saved);
      expect(session.projectJson, saved);
      expect(session.projectRevision, 1);
      expect(store.expectedHeads[1], firstHead);

      final value = await session.deriveAndSave<String>((latest) {
        expect(latest, saved);
        return ManagedProjectDerivedCandidate<String>(
          projectJson: derived,
          value: 'published',
        );
      });
      expect(value, 'published');
      expect(session.projectJson, derived);
      expect(session.projectRevision, 2);
      final exactFinalHead = session.head.canonicalJson;
      await session.close();
      await session.close();

      final reopened = await ManagedRevision3AuthoringProjectSession.open(
        root: root,
        store: store,
      );
      expect(reopened.head.canonicalJson, exactFinalHead);
      expect(reopened.projectJson, derived);
      expect(reopened.projectRevision, 2);
      expect(
        store.openVerifications,
        everyElement(AuthoringAssetVerification.full),
      );
      expect(
        store.headVerifications,
        everyElement(AuthoringAssetVerification.full),
      );
      await reopened.close();
    },
  );

  test(
    'queued Quest transactions bind latest R3 basis and publish fully reopened candidates',
    () async {
      final root = await _projectRoot(fixture);
      final store = _FakeRevision3Store();
      final original = _projectJson(revision: 0, name: 'Quest project');
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: original,
      );
      final originalHead = session.head.canonicalJson;
      final genericPrepares = store.prepareCalls;

      final first = session.prepareAndPublishQuestDraftV3(
        gameRoot: r'D:\Games\Gothic Remake',
        questId: '00000000000000000000000000000071',
        scriptModuleId: '00000000000000000000000000000072',
        displayName: 'Managed Quest 1',
        intent: _questIntent(1),
      );
      final second = session.prepareAndPublishQuestDraftV3(
        gameRoot: r'D:\Games\Gothic Remake',
        questId: '00000000000000000000000000000073',
        scriptModuleId: '00000000000000000000000000000074',
        displayName: 'Managed Quest 2',
        intent: _questIntent(2),
      );
      final results = await Future.wait(
        <Future<ManagedRevision3QuestDraftCheckpoint>>[first, second],
      );

      expect(store.prepareCalls, genericPrepares);
      expect(store.questPrepareCalls, 2);
      expect(store.questCurrentProjects[0], original);
      expect(store.questRequests[0].expectedHead.canonicalJson, originalHead);
      expect(store.questRequests[0].expectedRevision, 0);
      expect(
        store.questRequests[1].expectedHead.canonicalJson,
        results[0].head.canonicalJson,
      );
      expect(store.questRequests[1].expectedRevision, 1);
      expect(store.questGameRoots, <String>[
        r'D:\Games\Gothic Remake',
        r'D:\Games\Gothic Remake',
      ]);
      expect(results[0].projectRevision, 1);
      expect(results[0].questId, '00000000000000000000000000000071');
      expect(results[1].projectRevision, 2);
      expect(results[1].questId, '00000000000000000000000000000073');
      expect(session.projectJson, results[1].projectJson);
      expect(session.projectRevision, 2);
      expect(session.head.canonicalJson, results[1].head.canonicalJson);
      expect(
        await session.headFile.readAsString(),
        results[1].head.canonicalJson,
      );
      expect(
        store.headVerifications,
        everyElement(AuthoringAssetVerification.full),
      );
      expect(
        store.openVerifications,
        everyElement(AuthoringAssetVerification.full),
      );
      await session.close();

      final reopened = await ManagedRevision3AuthoringProjectSession.open(
        root: root,
        store: store,
      );
      expect(reopened.projectJson, results[1].projectJson);
      expect(reopened.projectRevision, 2);
      await reopened.close();
    },
  );

  test(
    'concurrent saves run in invocation order with present-head CAS',
    () async {
      final root = await _projectRoot(fixture);
      final store = _FakeRevision3Store();
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: _projectJson(revision: 0, name: 'Original'),
      );
      final firstEntered = Completer<void>();
      final releaseFirst = Completer<void>();
      store.afterPrepare = (_, _, projectJson) async {
        expect(projectJson, _projectJson(revision: 1, name: 'First'));
        firstEntered.complete();
        await releaseFirst.future;
      };

      final first = session.save(_projectJson(revision: 1, name: 'First'));
      await firstEntered.future;
      final second = session.save(_projectJson(revision: 2, name: 'Second'));
      await Future<void>.delayed(Duration.zero);
      expect(store.prepareCalls, 2); // create plus the blocked first save
      releaseFirst.complete();
      await Future.wait(<Future<void>>[first, second]);

      expect(session.projectJson, _projectJson(revision: 2, name: 'Second'));
      expect(store.expectedHeads, hasLength(3));
      expect(store.expectedHeads[0], isNull);
      expect(store.expectedHeads[1], isNotNull);
      expect(store.expectedHeads[2], isNotNull);
      expect(store.expectedHeads[2], isNot(store.expectedHeads[1]));
      await session.close();
    },
  );

  test(
    'verifyCurrentHead performs one full reopen without prepare or publish',
    () async {
      final root = await _projectRoot(fixture);
      final store = _FakeRevision3Store();
      final project = _projectJson(revision: 0, name: 'Verified');
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: project,
      );
      final exactHead = session.head.canonicalJson;
      final prepareCalls = store.prepareCalls;
      final openCalls = store.openVerifications.length;
      final headOpenCalls = store.headVerifications.length;
      final headBytes = await session.headFile.readAsBytes();

      await session.verifyCurrentHead();

      expect(store.prepareCalls, prepareCalls);
      expect(store.openVerifications.length, openCalls + 1);
      expect(store.openVerifications.last, AuthoringAssetVerification.full);
      expect(store.headVerifications.length, headOpenCalls);
      expect(session.head.canonicalJson, exactHead);
      expect(session.projectJson, project);
      expect(await session.headFile.readAsBytes(), headBytes);
      expect(session.requiresReopen, isFalse);
      await session.close();
    },
  );

  test(
    'content read stays exact-head, serialized, and publication-free',
    () async {
      final root = await _projectRoot(fixture, suffix: 'content_exact');
      final store = _FakeRevision3Store();
      final original = _projectJson(revision: 0, name: 'Content exact');
      final saved = _projectJson(revision: 1, name: 'After content');
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: original,
      );
      final prepareCalls = store.prepareCalls;
      final openCalls = store.openVerifications.length;
      final headOpenCalls = store.headVerifications.length;
      final exactHead = session.head.canonicalJson;
      final headBytes = await session.headFile.readAsBytes();
      final readEntered = Completer<void>();
      final releaseRead = Completer<void>();
      final savePrepareEntered = Completer<void>();
      final releaseSavePrepare = Completer<void>();
      store.afterContentRead = (_, _, _) async {
        readEntered.complete();
        await releaseRead.future;
      };
      store.afterPrepare = (_, _, _) async {
        savePrepareEntered.complete();
        await releaseSavePrepare.future;
      };

      final reading = session.readContentIndex();
      await readEntered.future;
      final saving = session.save(saved);
      await Future<void>.delayed(Duration.zero);
      expect(store.prepareCalls, prepareCalls);
      expect(store.openVerifications.length, openCalls);
      expect(store.headVerifications.length, headOpenCalls);

      releaseRead.complete();
      final index = await reading;
      await savePrepareEntered.future;
      expect(await session.headFile.readAsBytes(), orderedEquals(headBytes));
      expect(session.projectJson, original);
      releaseSavePrepare.complete();
      await saving;

      expect(index.projectId, '00000000000000000000000000000003');
      expect(index.projectRevision, 0);
      expect(index.projectName, 'Content exact');
      expect(store.contentReadCalls, 1);
      expect(store.contentExpectedHeads, <String>[exactHead]);
      expect(store.prepareCalls, prepareCalls + 1);
      expect(session.projectJson, saved);
      expect(session.projectRevision, 1);
      expect(
        await session.headFile.readAsBytes(),
        isNot(orderedEquals(headBytes)),
      );
      expect(session.requiresReopen, isFalse);
      await session.close();
    },
  );

  test('content read head drift poisons without publishing', () async {
    final root = await _projectRoot(fixture, suffix: 'content_drift');
    final store = _FakeRevision3Store();
    final original = _projectJson(revision: 0, name: 'Content drift');
    final session = await ManagedRevision3AuthoringProjectSession.create(
      root: root,
      store: store,
      projectJson: original,
    );
    final prepareCalls = store.prepareCalls;
    final external = store.register(
      _projectJson(revision: 91, name: 'External content winner'),
    );
    store.afterContentRead = (rootPath, _, _) => File(
      p.join(rootPath, 'gore-project.json'),
    ).writeAsString(external.canonicalJson, flush: true);

    await expectLater(
      session.readContentIndex(),
      throwsA(isA<ManagedProjectHeadConflictException>()),
    );

    expect(await session.headFile.readAsString(), external.canonicalJson);
    expect(store.prepareCalls, prepareCalls);
    expect(session.projectJson, original);
    expect(session.requiresReopen, isTrue);
    await expectLater(
      session.readContentIndex(),
      throwsA(isA<ManagedProjectVerificationException>()),
    );
    expect(store.contentReadCalls, 1);
    await session.close();
  });

  test(
    'content capacity rejection is retryable while integrity failures poison',
    () async {
      final retryRoot = await _projectRoot(fixture, suffix: 'content_retry');
      final retryStore = _FakeRevision3Store();
      final retrySession = await ManagedRevision3AuthoringProjectSession.create(
        root: retryRoot,
        store: retryStore,
        projectJson: _projectJson(revision: 0, name: 'Content retry'),
      );
      retryStore.nextContentError = const ModFfiException(
        command: 'authoring_store_read_revision3_content_index_v1',
        code: 'AUTHORING_REVISION3_CONTENT_RESPONSE_LIMIT',
        message: 'fake bounded content limit',
      );
      await expectLater(
        retrySession.readContentIndex(),
        throwsA(
          isA<ModFfiException>().having(
            (error) => error.code,
            'code',
            'AUTHORING_REVISION3_CONTENT_RESPONSE_LIMIT',
          ),
        ),
      );
      expect(retrySession.requiresReopen, isFalse);
      expect((await retrySession.readContentIndex()).projectRevision, 0);
      await retrySession.close();

      final poisonRoot = await _projectRoot(fixture, suffix: 'content_poison');
      final poisonStore = _FakeRevision3Store();
      final poisonSession =
          await ManagedRevision3AuthoringProjectSession.create(
            root: poisonRoot,
            store: poisonStore,
            projectJson: _projectJson(revision: 0, name: 'Content poison'),
          );
      poisonStore.nextContentError = const ModFfiException(
        command: 'authoring_store_read_revision3_content_index_v1',
        code: 'AUTHORING_REVISION3_CONTENT_STORE_SEAL_MISMATCH',
        message: 'fake content integrity failure',
      );
      await expectLater(
        poisonSession.readContentIndex(),
        throwsA(isA<ManagedProjectVerificationException>()),
      );
      expect(poisonSession.requiresReopen, isTrue);
      await poisonSession.close();
    },
  );

  test('content response identity mismatch fails closed', () async {
    final root = await _projectRoot(fixture, suffix: 'content_identity');
    final store = _FakeRevision3Store();
    final session = await ManagedRevision3AuthoringProjectSession.create(
      root: root,
      store: store,
      projectJson: _projectJson(revision: 0, name: 'Content identity'),
    );
    final exactHead = await session.headFile.readAsBytes();
    store.nextContentResponseMismatch = 'project-id';

    await expectLater(
      session.readContentIndex(),
      throwsA(isA<ManagedProjectVerificationException>()),
    );

    expect(await session.headFile.readAsBytes(), exactHead);
    expect(session.requiresReopen, isTrue);
    await session.close();
  });

  test(
    'verifyCurrentHead drift or reopen mismatch poisons the session',
    () async {
      for (final mode in <String>['head-drift', 'reopen-mismatch']) {
        final root = await _projectRoot(fixture, suffix: mode);
        final store = _FakeRevision3Store();
        final original = _projectJson(revision: 0, name: 'Original $mode');
        final session = await ManagedRevision3AuthoringProjectSession.create(
          root: root,
          store: store,
          projectJson: original,
        );
        final prepareCalls = store.prepareCalls;
        if (mode == 'head-drift') {
          final external = store.register(
            _projectJson(revision: 91, name: 'External'),
          );
          await session.headFile.writeAsString(
            external.canonicalJson,
            flush: true,
          );
        } else {
          store.nextOpenProjectOverride = _projectJson(
            revision: 92,
            name: 'Wrong reopen',
          );
        }

        await expectLater(
          session.verifyCurrentHead(),
          throwsA(isA<ManagedProjectSessionException>()),
          reason: mode,
        );
        expect(store.prepareCalls, prepareCalls, reason: mode);
        expect(session.projectJson, original, reason: mode);
        expect(session.requiresReopen, isTrue, reason: mode);
        await expectLater(
          session.verifyCurrentHead(),
          throwsA(isA<ManagedProjectVerificationException>()),
          reason: mode,
        );
        await session.close();
      }
    },
  );

  test(
    'derive rejection and callback throw prepare and publish nothing',
    () async {
      final root = await _projectRoot(fixture);
      final store = _FakeRevision3Store();
      final original = _projectJson(revision: 0, name: 'Original');
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: original,
      );
      final prepares = store.prepareCalls;
      final exactHead = await session.headFile.readAsBytes();

      final rejection = await session.deriveAndSave<int>((latest) {
        expect(latest, original);
        return const ManagedProjectDerivedRejection<int>(41);
      });
      expect(rejection, 41);
      expect(store.prepareCalls, prepares);
      expect(await session.headFile.readAsBytes(), exactHead);

      await expectLater(
        session.deriveAndSave<void>((_) => throw StateError('derive failed')),
        throwsA(isA<StateError>()),
      );
      expect(store.prepareCalls, prepares);
      expect(await session.headFile.readAsBytes(), exactHead);
      expect(session.requiresReopen, isFalse);
      await session.close();
    },
  );

  test(
    'an external race never clobbers and poisons edits until reopen',
    () async {
      final root = await _projectRoot(fixture);
      final store = _FakeRevision3Store();
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: _projectJson(revision: 0, name: 'Original'),
      );
      final externalProject = _projectJson(revision: 90, name: 'External');
      final externalHead = store.register(externalProject);
      store.afterPrepare = (rootPath, _, _) => File(
        p.join(rootPath, 'gore-project.json'),
      ).writeAsString(externalHead.canonicalJson, flush: true);

      await expectLater(
        session.save(_projectJson(revision: 1, name: 'Must not win')),
        throwsA(isA<ManagedProjectHeadConflictException>()),
      );
      expect(await session.headFile.readAsString(), externalHead.canonicalJson);
      expect(session.requiresReopen, isTrue);
      final prepares = store.prepareCalls;
      await expectLater(
        session.save(_projectJson(revision: 2, name: 'Still rejected')),
        throwsA(isA<ManagedProjectVerificationException>()),
      );
      expect(store.prepareCalls, prepares);
      await session.close();

      final reopened = await ManagedRevision3AuthoringProjectSession.open(
        root: root,
        store: store,
      );
      expect(reopened.projectJson, externalProject);
      expect(reopened.requiresReopen, isFalse);
      await reopened.close();
    },
  );

  test(
    'interrupted publication is repaired by a full verified reopen',
    () async {
      final root = await _projectRoot(fixture);
      final store = _FakeRevision3Store();
      var armed = false;
      final replacement = AtomicByteReplacement(
        operationIdFactory: () => '73000000000000000000000000000001',
        onPhase: (phase) {
          if (armed && phase == AtomicSwapPhase.tempPromoted) {
            throw const AtomicSwapException('injected publication failure');
          }
        },
      );
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: _projectJson(revision: 0, name: 'Original'),
        replacement: replacement,
      );
      armed = true;
      final saved = _projectJson(revision: 1, name: 'Recovered');

      await expectLater(
        session.save(saved),
        throwsA(isA<AtomicSwapException>()),
      );
      expect(session.requiresReopen, isTrue);
      final journal = File(
        AtomicByteReplacement.journalPathFor(session.headFile),
      );
      expect(await journal.exists(), isTrue);
      await session.close();

      final reopened = await ManagedRevision3AuthoringProjectSession.open(
        root: root,
        store: store,
      );
      expect(reopened.projectJson, saved);
      expect(await journal.exists(), isFalse);
      expect(
        store.headVerifications,
        everyElement(AuthoringAssetVerification.full),
      );
      await reopened.close();
    },
  );

  test(
    'interrupted Quest publication repairs the verified prepared candidate',
    () async {
      final root = await _projectRoot(fixture, suffix: 'quest_repair');
      final store = _FakeRevision3Store();
      var armed = false;
      final replacement = AtomicByteReplacement(
        operationIdFactory: () => '75000000000000000000000000000001',
        onPhase: (phase) {
          if (armed && phase == AtomicSwapPhase.tempPromoted) {
            throw const AtomicSwapException(
              'injected Quest publication failure',
            );
          }
        },
      );
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: _projectJson(revision: 0, name: 'Quest repair'),
        replacement: replacement,
      );
      armed = true;

      await expectLater(
        session.prepareAndPublishQuestDraftV3(
          gameRoot: r'D:\Games\Gothic Remake',
          questId: '00000000000000000000000000000071',
          scriptModuleId: '00000000000000000000000000000072',
          displayName: 'Managed Quest 1',
          intent: _questIntent(1),
        ),
        throwsA(isA<AtomicSwapException>()),
      );
      expect(session.projectRevision, 0);
      expect(session.requiresReopen, isTrue);
      final journal = File(
        AtomicByteReplacement.journalPathFor(session.headFile),
      );
      expect(await journal.exists(), isTrue);
      await session.close();

      final reopened = await ManagedRevision3AuthoringProjectSession.open(
        root: root,
        store: store,
      );
      expect(reopened.projectRevision, 1);
      expect(await journal.exists(), isFalse);
      expect(
        store.headVerifications,
        everyElement(AuthoringAssetVerification.full),
      );
      await reopened.close();
    },
  );

  test(
    'raw filesystem publication failure poisons and repairs on reopen',
    () async {
      final root = await _projectRoot(fixture);
      final store = _FakeRevision3Store();
      var armed = false;
      final replacement = AtomicByteReplacement(
        operationIdFactory: () => '74000000000000000000000000000001',
        onPhase: (phase) {
          if (armed && phase == AtomicSwapPhase.targetBackedUp) {
            throw FileSystemException('injected raw publication failure');
          }
        },
      );
      final original = _projectJson(revision: 0, name: 'Original');
      final saved = _projectJson(revision: 1, name: 'Recovered raw failure');
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: original,
        replacement: replacement,
      );
      final originalHead = session.head.canonicalJson;
      armed = true;

      await expectLater(
        session.save(saved),
        throwsA(isA<FileSystemException>()),
      );
      expect(session.requiresReopen, isTrue);
      expect(session.projectJson, original);
      expect(session.head.canonicalJson, originalHead);
      expect(await session.headFile.exists(), isFalse);
      final journal = File(
        AtomicByteReplacement.journalPathFor(session.headFile),
      );
      expect(await journal.exists(), isTrue);

      final preparesAfterFailure = store.prepareCalls;
      await expectLater(
        session.save(_projectJson(revision: 2, name: 'Must not prepare')),
        throwsA(isA<ManagedProjectVerificationException>()),
      );
      expect(store.prepareCalls, preparesAfterFailure);
      await session.close();

      final reopened = await ManagedRevision3AuthoringProjectSession.open(
        root: root,
        store: store,
      );
      expect(reopened.projectJson, saved);
      expect(reopened.projectRevision, 1);
      expect(reopened.requiresReopen, isFalse);
      expect(
        await reopened.headFile.readAsString(),
        reopened.head.canonicalJson,
      );
      expect(await journal.exists(), isFalse);
      expect(
        store.headVerifications,
        everyElement(AuthoringAssetVerification.full),
      );
      await reopened.close();
    },
  );

  test(
    'post-publication mismatch poisons session but reopen recovers',
    () async {
      final root = await _projectRoot(fixture);
      final store = _FakeRevision3Store();
      final original = _projectJson(revision: 0, name: 'Original');
      final saved = _projectJson(revision: 1, name: 'Saved');
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: original,
      );
      store.nextOpenProjectOverride = _projectJson(
        revision: 99,
        name: 'Mismatch',
      );

      await expectLater(
        session.save(saved),
        throwsA(isA<ManagedProjectVerificationException>()),
      );
      expect(session.projectJson, original);
      expect(session.requiresReopen, isTrue);
      await session.close();

      final reopened = await ManagedRevision3AuthoringProjectSession.open(
        root: root,
        store: store,
      );
      expect(reopened.projectJson, saved);
      await reopened.close();
    },
  );

  test(
    'prepared candidate head and project mismatches never publish',
    () async {
      for (final mismatch in <String>['head', 'project']) {
        final root = await _projectRoot(fixture, suffix: mismatch);
        final store = _FakeRevision3Store();
        final original = _projectJson(revision: 0, name: 'Original $mismatch');
        final candidate = _projectJson(
          revision: 1,
          name: 'Candidate $mismatch',
        );
        final session = await ManagedRevision3AuthoringProjectSession.create(
          root: root,
          store: store,
          projectJson: original,
        );
        final exactPublishedHead = await session.headFile.readAsBytes();
        if (mismatch == 'head') {
          store.nextHeadOverride = store.register(
            _projectJson(revision: 88, name: 'Wrong head'),
          );
        } else {
          store.nextHeadProjectOverride = _projectJson(
            revision: 89,
            name: 'Wrong project',
          );
        }

        await expectLater(
          session.save(candidate),
          throwsA(isA<ManagedProjectVerificationException>()),
          reason: mismatch,
        );
        expect(await session.headFile.readAsBytes(), exactPublishedHead);
        expect(session.projectJson, original);
        expect(session.requiresReopen, isFalse);

        await session.save(candidate);
        expect(session.projectJson, candidate);
        await session.close();
      }
    },
  );

  test(
    'Quest basis, response, and full-reopen mismatches never publish',
    () async {
      for (final mismatch in <String>[
        'basis-head',
        'candidate-project',
        'revision',
        'display-name',
        'candidate-reopen',
      ]) {
        final root = await _projectRoot(fixture, suffix: 'quest_$mismatch');
        final store = _FakeRevision3Store();
        final original = _projectJson(revision: 0, name: 'Quest $mismatch');
        final session = await ManagedRevision3AuthoringProjectSession.create(
          root: root,
          store: store,
          projectJson: original,
        );
        final exactHeadBytes = await session.headFile.readAsBytes();
        final genericPrepares = store.prepareCalls;
        if (mismatch == 'candidate-reopen') {
          store.nextHeadOverride = store.register(
            _projectJson(revision: 70, name: 'Wrong candidate reopen'),
          );
        } else {
          store.nextQuestResponseMismatch = mismatch;
        }

        await expectLater(
          session.prepareAndPublishQuestDraftV3(
            gameRoot: r'D:\Games\Gothic Remake',
            questId: '00000000000000000000000000000071',
            scriptModuleId: '00000000000000000000000000000072',
            displayName: 'Managed Quest 1',
            intent: _questIntent(1),
          ),
          throwsA(
            anyOf(
              isA<FormatException>(),
              isA<ManagedProjectVerificationException>(),
            ),
          ),
          reason: mismatch,
        );
        expect(
          await session.headFile.readAsBytes(),
          exactHeadBytes,
          reason: mismatch,
        );
        expect(session.projectJson, original, reason: mismatch);
        expect(session.requiresReopen, isFalse, reason: mismatch);
        expect(store.prepareCalls, genericPrepares, reason: mismatch);

        final published = await session.prepareAndPublishQuestDraftV3(
          gameRoot: r'D:\Games\Gothic Remake',
          questId: '00000000000000000000000000000071',
          scriptModuleId: '00000000000000000000000000000072',
          displayName: 'Managed Quest 1',
          intent: _questIntent(1),
        );
        expect(session.head.canonicalJson, published.head.canonicalJson);
        await session.close();
      }
    },
  );

  test(
    'Quest semantic rejection is retryable while native integrity errors poison',
    () async {
      final semanticRoot = await _projectRoot(
        fixture,
        suffix: 'quest_semantic',
      );
      final semanticStore = _FakeRevision3Store();
      final semanticSession =
          await ManagedRevision3AuthoringProjectSession.create(
            root: semanticRoot,
            store: semanticStore,
            projectJson: _projectJson(revision: 0, name: 'Quest semantic'),
          );
      final semanticHead = await semanticSession.headFile.readAsBytes();
      semanticStore.nextQuestError = const ModFfiException(
        command: 'authoring_store_prepare_revision3_quest_draft_v3',
        code: 'AUTHORING_REVISION3_QUEST_REJECTED',
        message: 'fake semantic collision',
      );
      await expectLater(
        semanticSession.prepareAndPublishQuestDraftV3(
          gameRoot: r'D:\Games\Gothic Remake',
          questId: '00000000000000000000000000000071',
          scriptModuleId: '00000000000000000000000000000072',
          displayName: 'Managed Quest 1',
          intent: _questIntent(1),
        ),
        throwsA(
          isA<ModFfiException>().having(
            (error) => error.code,
            'code',
            'AUTHORING_REVISION3_QUEST_REJECTED',
          ),
        ),
      );
      expect(await semanticSession.headFile.readAsBytes(), semanticHead);
      expect(semanticSession.requiresReopen, isFalse);
      await semanticSession.prepareAndPublishQuestDraftV3(
        gameRoot: r'D:\Games\Gothic Remake',
        questId: '00000000000000000000000000000071',
        scriptModuleId: '00000000000000000000000000000072',
        displayName: 'Managed Quest 1',
        intent: _questIntent(1),
      );
      await semanticSession.close();

      for (final errorCode in <String>[
        'AUTHORING_REVISION3_QUEST_HEAD_CONFLICT',
        'AUTHORING_REVISION3_QUEST_STORE_SEAL_MISMATCH',
      ]) {
        final root = await _projectRoot(
          fixture,
          suffix: errorCode.toLowerCase(),
        );
        final store = _FakeRevision3Store();
        final session = await ManagedRevision3AuthoringProjectSession.create(
          root: root,
          store: store,
          projectJson: _projectJson(revision: 0, name: errorCode),
        );
        final exactHead = await session.headFile.readAsBytes();
        store.nextQuestError = ModFfiException(
          command: 'authoring_store_prepare_revision3_quest_draft_v3',
          code: errorCode,
          message: 'fake integrity failure',
        );
        await expectLater(
          session.prepareAndPublishQuestDraftV3(
            gameRoot: r'D:\Games\Gothic Remake',
            questId: '00000000000000000000000000000071',
            scriptModuleId: '00000000000000000000000000000072',
            displayName: 'Managed Quest 1',
            intent: _questIntent(1),
          ),
          throwsA(
            errorCode.endsWith('HEAD_CONFLICT')
                ? isA<ManagedProjectHeadConflictException>()
                : isA<ManagedProjectVerificationException>(),
          ),
          reason: errorCode,
        );
        expect(await session.headFile.readAsBytes(), exactHead);
        expect(session.requiresReopen, isTrue);
        final questCalls = store.questPrepareCalls;
        await expectLater(
          session.prepareAndPublishQuestDraftV3(
            gameRoot: r'D:\Games\Gothic Remake',
            questId: '00000000000000000000000000000073',
            scriptModuleId: '00000000000000000000000000000074',
            displayName: 'Managed Quest 2',
            intent: _questIntent(2),
          ),
          throwsA(isA<ManagedProjectVerificationException>()),
        );
        expect(store.questPrepareCalls, questCalls);
        await session.close();
      }
    },
  );

  test(
    'head drift during native Quest prepare never clobbers the winner',
    () async {
      final root = await _projectRoot(fixture, suffix: 'quest_race');
      final store = _FakeRevision3Store();
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: _projectJson(revision: 0, name: 'Quest race'),
      );
      final externalProject = _projectJson(
        revision: 90,
        name: 'External winner',
      );
      final externalHead = store.register(externalProject);
      store.afterQuestPrepare = (rootPath, _, _, _) => File(
        p.join(rootPath, 'gore-project.json'),
      ).writeAsString(externalHead.canonicalJson, flush: true);

      await expectLater(
        session.prepareAndPublishQuestDraftV3(
          gameRoot: r'D:\Games\Gothic Remake',
          questId: '00000000000000000000000000000071',
          scriptModuleId: '00000000000000000000000000000072',
          displayName: 'Managed Quest 1',
          intent: _questIntent(1),
        ),
        throwsA(isA<ManagedProjectHeadConflictException>()),
      );
      expect(await session.headFile.readAsString(), externalHead.canonicalJson);
      expect(session.projectRevision, 0);
      expect(session.requiresReopen, isTrue);
      await session.close();

      final reopened = await ManagedRevision3AuthoringProjectSession.open(
        root: root,
        store: store,
      );
      expect(reopened.projectJson, externalProject);
      await reopened.close();
    },
  );

  test(
    'derive callback cannot re-enter save, derive, verify, or close',
    () async {
      final root = await _projectRoot(fixture);
      final store = _FakeRevision3Store();
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: _projectJson(revision: 0, name: 'Original'),
      );

      final result = await session.deriveAndSave<String>((_) async {
        await expectLater(
          session.save(_projectJson(revision: 1, name: 'Nested')),
          throwsA(isA<ManagedProjectReentrantOperationException>()),
        );
        await expectLater(
          session.deriveAndSave<void>(
            (_) => const ManagedProjectDerivedRejection<void>(null),
          ),
          throwsA(isA<ManagedProjectReentrantOperationException>()),
        );
        await expectLater(
          session.verifyCurrentHead(),
          throwsA(isA<ManagedProjectReentrantOperationException>()),
        );
        await expectLater(
          session.readContentIndex(),
          throwsA(isA<ManagedProjectReentrantOperationException>()),
        );
        await expectLater(
          session.prepareAndPublishQuestDraftV3(
            gameRoot: r'D:\Games\Gothic Remake',
            questId: '00000000000000000000000000000071',
            scriptModuleId: '00000000000000000000000000000072',
            displayName: 'Managed Quest 1',
            intent: _questIntent(1),
          ),
          throwsA(isA<ManagedProjectReentrantOperationException>()),
        );
        await expectLater(
          session.close(),
          throwsA(isA<ManagedProjectReentrantOperationException>()),
        );
        return const ManagedProjectDerivedRejection<String>('closed');
      });
      expect(result, 'closed');
      expect(session.isClosed, isFalse);
      await session.close();
    },
  );

  test(
    'close waits for prior work, rejects new work, and releases lock',
    () async {
      final root = await _projectRoot(fixture);
      final store = _FakeRevision3Store();
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: _projectJson(revision: 0, name: 'Original'),
      );
      final entered = Completer<void>();
      final release = Completer<void>();
      store.afterPrepare = (_, _, _) async {
        entered.complete();
        await release.future;
      };
      final save = session.save(_projectJson(revision: 1, name: 'Saved'));
      await entered.future;
      final close = session.close();
      await expectLater(
        session.save(_projectJson(revision: 2, name: 'Too late')),
        throwsA(isA<ManagedProjectSessionClosedException>()),
      );
      expect(session.isClosed, isFalse);
      release.complete();
      await save;
      await close;
      expect(session.isClosed, isTrue);

      final lock = await ManagedProjectSessionLock.acquire(root);
      await lock.release();
    },
  );

  test('one managed root has one exclusive R3 session', () async {
    final root = await _projectRoot(fixture);
    final store = _FakeRevision3Store();
    final first = await ManagedRevision3AuthoringProjectSession.create(
      root: root,
      store: store,
      projectJson: _projectJson(revision: 0, name: 'Original'),
    );

    await expectLater(
      ManagedRevision3AuthoringProjectSession.open(root: root, store: store),
      throwsA(isA<ManagedProjectLockException>()),
    );
    await first.close();
  });
}

typedef _AfterPrepare =
    FutureOr<void> Function(
      String root,
      AuthoringWorkingHead head,
      String projectJson,
    );

typedef _AfterQuestPrepare =
    FutureOr<void> Function(
      String root,
      AuthoringWorkingHead basisHead,
      AuthoringWorkingHead candidateHead,
      String candidateProjectJson,
    );

typedef _AfterContentRead =
    FutureOr<void> Function(
      String root,
      AuthoringWorkingHead expectedHead,
      String projectJson,
    );

final class _FakeRevision3Store implements ManagedRevision3AuthoringStore {
  final Map<String, String> _projectsByHead = <String, String>{};
  final List<AuthoringAssetVerification> openVerifications =
      <AuthoringAssetVerification>[];
  final List<AuthoringAssetVerification> headVerifications =
      <AuthoringAssetVerification>[];
  final List<String?> expectedHeads = <String?>[];
  int _sequence = 0;
  int prepareCalls = 0;
  int questPrepareCalls = 0;
  int contentReadCalls = 0;
  _AfterPrepare? afterPrepare;
  _AfterQuestPrepare? afterQuestPrepare;
  _AfterContentRead? afterContentRead;
  final List<String> questGameRoots = <String>[];
  final List<String> questCurrentProjects = <String>[];
  final List<AuthoringRevision3QuestDraftRequestV3> questRequests =
      <AuthoringRevision3QuestDraftRequestV3>[];
  String? nextQuestResponseMismatch;
  ModFfiException? nextQuestError;
  ModFfiException? nextContentError;
  String? nextContentResponseMismatch;
  final List<String> contentExpectedHeads = <String>[];
  String? nextOpenProjectOverride;
  AuthoringWorkingHead? nextHeadOverride;
  String? nextHeadProjectOverride;

  AuthoringWorkingHead register(String projectJson) {
    _sequence++;
    final sha = _sequence.toRadixString(16).padLeft(64, '0');
    final head = AuthoringWorkingHead.fromCanonicalJson(
      jsonEncode(<String, Object?>{
        'store_format': 1,
        'snapshot': <String, Object?>{
          'byte_len': utf8.encode(projectJson).length,
          'sha256': sha,
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
    openVerifications.add(verification);
    final rawHead = await File(
      p.join(root, 'gore-project.json'),
    ).readAsString();
    final head = AuthoringWorkingHead.fromCanonicalJson(rawHead);
    final project = _projectsByHead[rawHead];
    if (project == null) throw StateError('unknown published head');
    final override = nextOpenProjectOverride;
    nextOpenProjectOverride = null;
    return AuthoringRevision3StoreOpenedResult.fromJson(
      _openedResponse(head, override ?? project),
    );
  }

  @override
  Future<AuthoringRevision3StoreOpenedResult> openHeadBytes({
    required String root,
    required AuthoringWorkingHead head,
    required AuthoringAssetVerification verification,
  }) async {
    headVerifications.add(verification);
    final project = _projectsByHead[head.canonicalJson];
    if (project == null) throw StateError('unknown checkpoint head');
    final headOverride = nextHeadOverride;
    nextHeadOverride = null;
    final projectOverride = nextHeadProjectOverride;
    nextHeadProjectOverride = null;
    return AuthoringRevision3StoreOpenedResult.fromJson(
      _openedResponse(headOverride ?? head, projectOverride ?? project),
    );
  }

  @override
  Future<AuthoringRevision3CheckpointPreparation> prepareCheckpoint({
    required String root,
    required AuthoringWorkingHead? expectedHead,
    required String projectJson,
  }) async {
    prepareCalls++;
    expectedHeads.add(expectedHead?.canonicalJson);
    final headFile = File(p.join(root, 'gore-project.json'));
    final actual = await headFile.exists()
        ? await headFile.readAsString()
        : null;
    if (actual != expectedHead?.canonicalJson) {
      throw const ModFfiException(
        command: 'authoring_store_prepare_revision3_checkpoint',
        code: 'AUTHORING_STORE_HEAD_CONFLICT',
        message: 'fake native head CAS rejected',
      );
    }
    final head = register(projectJson);
    final hook = afterPrepare;
    afterPrepare = null;
    await hook?.call(root, head, projectJson);
    return AuthoringRevision3CheckpointPreparation.fromJson(
      _preparedResponse(head),
    );
  }

  @override
  Future<AuthoringRevision3QuestDraftPreparation> prepareQuestDraftV3({
    required String root,
    required String gameRoot,
    required String currentProjectJson,
    required String questRequestJson,
  }) async {
    questPrepareCalls++;
    questGameRoots.add(gameRoot);
    questCurrentProjects.add(currentProjectJson);
    final request = AuthoringRevision3QuestDraftRequestV3.fromCanonicalJson(
      questRequestJson,
    );
    questRequests.add(request);
    final injectedError = nextQuestError;
    nextQuestError = null;
    if (injectedError != null) throw injectedError;

    final headFile = File(p.join(root, 'gore-project.json'));
    final actual = await headFile.readAsString();
    if (actual != request.expectedHead.canonicalJson ||
        _projectsByHead[actual] != currentProjectJson) {
      throw const ModFfiException(
        command: 'authoring_store_prepare_revision3_quest_draft_v3',
        code: 'AUTHORING_REVISION3_QUEST_HEAD_CONFLICT',
        message: 'fake native Quest basis CAS rejected',
      );
    }
    final basis = jsonDecode(currentProjectJson) as Map<String, Object?>;
    if (basis['project_id'] != request.expectedProjectId ||
        basis['revision'] != request.expectedRevision) {
      throw const ModFfiException(
        command: 'authoring_store_prepare_revision3_quest_draft_v3',
        code: 'AUTHORING_REVISION3_QUEST_HEAD_CONFLICT',
        message: 'fake native Quest project binding rejected',
      );
    }
    final projectId = request.expectedProjectId;
    final rawEntities = (basis['entities'] as Map).cast<String, Object?>();
    final entities = SplayTreeMap<String, Object?>.from(rawEntities);
    final questInput = _questInput(
      request: request,
      basisHead: request.expectedHead,
      target: (basis['target'] as Map).cast<String, Object?>(),
    );
    entities[request.questId] = _questEntity(
      projectId: projectId,
      request: request,
      input: questInput,
    );
    entities[request.scriptModuleId] = _questModuleEntity(
      projectId: projectId,
      request: request,
      input: questInput,
    );
    basis['revision'] = request.expectedRevision + 1;
    basis['entities'] = entities;
    final assetStore = (basis['asset_store'] as Map).cast<String, Object?>();
    final assets = SplayTreeMap<String, Object?>.from(
      (assetStore['assets'] as Map).cast<String, Object?>(),
    );
    assets[_questArtifactSha] = <String, Object?>{
      'byte_len': 123,
      'media_type':
          'application/vnd.gore.quest-collision-capability+json;version=2',
    };
    assetStore['assets'] = assets;
    basis['asset_store'] = assetStore;
    var candidateProject = jsonEncode(basis);
    var candidateHead = register(candidateProject);
    final hook = afterQuestPrepare;
    afterQuestPrepare = null;
    await hook?.call(
      root,
      request.expectedHead,
      candidateHead,
      candidateProject,
    );

    final mismatch = nextQuestResponseMismatch;
    nextQuestResponseMismatch = null;
    var basisHead = request.expectedHead;
    var responseRevision = request.expectedRevision + 1;
    var responseQuestId = request.questId;
    var responseModuleId = request.scriptModuleId;
    if (mismatch == 'basis-head') {
      basisHead = register(_projectJson(revision: 81, name: 'Wrong basis'));
    } else if (mismatch == 'candidate-project') {
      candidateProject = candidateProject.replaceAll(
        projectId,
        '00000000000000000000000000000093',
      );
      candidateHead = register(candidateProject);
    } else if (mismatch == 'revision') {
      responseRevision++;
    } else if (mismatch == 'display-name') {
      candidateProject = candidateProject.replaceFirst(
        '"display_name":"${request.displayName}"',
        '"display_name":"Wrong prepared Quest"',
      );
      candidateHead = register(candidateProject);
    } else if (mismatch == 'quest-id') {
      responseQuestId = request.scriptModuleId;
    } else if (mismatch == 'module-id') {
      responseModuleId = request.questId;
    }
    return AuthoringRevision3QuestDraftPreparation.fromJson(
      _questPreparedResponse(
        basisHead: basisHead,
        candidateHead: candidateHead,
        candidateProjectJson: candidateProject,
        revision: responseRevision,
        questId: responseQuestId,
        scriptModuleId: responseModuleId,
      ),
    );
  }

  @override
  Future<AuthoringRevision3ContentIndexResult> readContentIndex({
    required String root,
    required AuthoringWorkingHead expectedHead,
  }) async {
    contentReadCalls++;
    contentExpectedHeads.add(expectedHead.canonicalJson);
    final injectedError = nextContentError;
    nextContentError = null;
    if (injectedError != null) throw injectedError;

    final actual = await File(p.join(root, 'gore-project.json')).readAsString();
    if (actual != expectedHead.canonicalJson) {
      throw const ModFfiException(
        command: 'authoring_store_read_revision3_content_index_v1',
        code: 'AUTHORING_REVISION3_CONTENT_HEAD_CONFLICT',
        message: 'fake native content head CAS rejected',
      );
    }
    final project = _projectsByHead[actual];
    if (project == null) throw StateError('unknown content checkpoint head');
    final hook = afterContentRead;
    afterContentRead = null;
    await hook?.call(root, expectedHead, project);

    final mismatch = nextContentResponseMismatch;
    nextContentResponseMismatch = null;
    return AuthoringRevision3ContentIndexResult.fromJson(
      _contentResponse(
        expectedHead,
        project,
        responseProjectId: mismatch == 'project-id'
            ? '93939393939393939393939393939393'
            : null,
      ),
      expectedHead: expectedHead,
    );
  }
}

Future<Directory> _projectRoot(Directory fixture, {String suffix = ''}) async {
  final root = Directory(
    p.join(fixture.path, suffix.isEmpty ? 'project' : 'project_$suffix'),
  );
  await root.create();
  return root;
}

String _projectJson({required int revision, required String name}) =>
    jsonEncode(<String, Object?>{
      'format': 2,
      'schema_revision': 3,
      'project_id': '00000000000000000000000000000003',
      'revision': revision,
      'meta': <String, Object?>{
        'name': name,
        'version': '1.0.0',
        'author': 'revision-3 session tests',
      },
      'target': <String, Object?>{
        'executable': <String, Object?>{
          'byte_len': 1,
          'sha256': List<String>.filled(64, '3').join(),
        },
      },
      'authoring_locales': <Object?>[],
      'entities': <String, Object?>{},
      'asset_store': <String, Object?>{'assets': <String, Object?>{}},
    });

Map<String, Object?> _contentResponse(
  AuthoringWorkingHead head,
  String projectJson, {
  String? responseProjectId,
}) {
  final project = jsonDecode(projectJson) as Map<String, Object?>;
  final projectId = responseProjectId ?? project['project_id']! as String;
  final revision = project['revision']! as int;
  final meta = (project['meta']! as Map).cast<String, Object?>();
  final target = (project['target']! as Map).cast<String, Object?>();
  return <String, Object?>{
    'ok': true,
    'head_json': head.canonicalJson,
    'project_id': projectId,
    'project_revision': revision,
    'index_json': jsonEncode(<String, Object?>{
      'schema_revision': 1,
      'project_id': projectId,
      'project_revision': revision,
      'project_name': meta['name'],
      'project_version': meta['version'],
      'project_author': meta['author'],
      'target': target,
      'authoring_locales': project['authoring_locales'],
      'entity_counts': <String, Object?>{},
      'entities': <Object?>[],
      'assets': <Object?>[],
    }),
    'content_authority': 'read_only_exact_current_project',
    'build_status': 'not_evaluated',
    'runtime_status': 'runtime_unqualified',
    'publication_status': 'not_applicable',
  };
}

const _questArtifactSha =
    'eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee';

Map<String, Object?> _questSeal(int byteLength, String digit) =>
    <String, Object?>{
      'byte_len': byteLength,
      'sha256': List<String>.filled(64, digit).join(),
    };

Map<String, Object?> _questInput({
  required AuthoringRevision3QuestDraftRequestV3 request,
  required AuthoringWorkingHead basisHead,
  required Map<String, Object?> target,
}) => <String, Object?>{
  'target': target,
  'quest_id': request.questId,
  'module_namespace': request.intent.moduleNamespace,
  'technical_id': request.intent.technicalId,
  'text_helper': request.intent.textHelper,
  'parent_quest': <String, Object?>{
    'generation': target,
    'source_seal': _questSeal(11, '1'),
    'catalog_layer': 'base-game.quest-parent.v1',
    'canonical_selector': 'SwampCamp_SCChapter2',
    'runtime_class': 'UQuest_SwampCamp_SCChapter2',
  },
  'giver': <String, Object?>{
    'generation': target,
    'source_seal': _questSeal(12, '2'),
    'catalog_layer': 'base-game.npc.v1',
    'canonical_selector': 'OM_GRD_Asghan_263',
    'runtime_unique_name': 'OM_GRD_Asghan_263',
  },
  'title': request.intent.title,
  'description': request.intent.description,
  'objective_title': request.intent.objectiveTitle,
  'collision_catalog': <String, Object?>{
    'generation': target,
    'catalog_layer':
        'base-game-plus-exact-revision3-project.story-collisions.v2',
    'artifact': _questSeal(123, 'e'),
    'source_seal': _questSeal(123, 'f'),
    'basis_snapshot': <String, Object?>{
      'byte_len': basisHead.snapshotByteLength,
      'sha256': basisHead.snapshotSha256,
    },
  },
};

String _questInputFingerprint(Map<String, Object?> input) {
  return revision3QuestInputFingerprint(input);
}

Map<String, Object?> _questEntity({
  required String projectId,
  required AuthoringRevision3QuestDraftRequestV3 request,
  required Map<String, Object?> input,
}) => <String, Object?>{
  'id': request.questId,
  'display_name': request.displayName,
  'origin': <String, Object?>{
    'type': 'new',
    'authored_runtime_id': request.intent.technicalId,
  },
  'revision': 0,
  'payload': <String, Object?>{
    'kind': 'quest_draft',
    'data': <String, Object?>{
      'generator_id': 'gore-authoring.draft-quest-skeleton',
      'generator_version': 2,
      'input': input,
      'script_module': <String, Object?>{
        'project_id': projectId,
        'id': request.scriptModuleId,
        'expected_kind': 'script_module',
      },
    },
  },
};

Map<String, Object?> _questModuleEntity({
  required String projectId,
  required AuthoringRevision3QuestDraftRequestV3 request,
  required Map<String, Object?> input,
}) {
  final source = revision3QuestGeneratedSource(
    technicalId: request.intent.technicalId,
    textHelper: request.intent.textHelper,
    parentRuntimeClass: 'UQuest_SwampCamp_SCChapter2',
    giverRuntimeUniqueName: 'OM_GRD_Asghan_263',
    title: request.intent.title,
    description: request.intent.description,
    objectiveTitle: request.intent.objectiveTitle,
  );
  return <String, Object?>{
    'id': request.scriptModuleId,
    'display_name': '${request.displayName} Script',
    'origin': <String, Object?>{
      'type': 'generated',
      'generator_id': 'gore-authoring.draft-quest-skeleton',
      'generator_version': 2,
      'owner': <String, Object?>{
        'project_id': projectId,
        'id': request.questId,
        'expected_kind': 'quest_draft',
      },
    },
    'revision': 0,
    'payload': <String, Object?>{
      'kind': 'script_module',
      'data': <String, Object?>{
        'generator_id': 'gore-authoring.draft-quest-skeleton',
        'generator_version': 2,
        'owner': <String, Object?>{
          'project_id': projectId,
          'id': request.questId,
          'expected_kind': 'quest_draft',
        },
        'module_namespace': request.intent.moduleNamespace,
        'module_relative_path':
            '${request.intent.moduleNamespace.replaceAll('.', '/')}.as',
        'source': source,
        'source_sha256': crypto.sha256.convert(utf8.encode(source)).toString(),
        'input_fingerprint': _questInputFingerprint(input),
        'status': <String, Object?>{
          'authoring': 'offline_draft',
          'runtime': 'runtime_unqualified',
        },
      },
    },
  };
}

Map<String, Object?> _questPreparedResponse({
  required AuthoringWorkingHead basisHead,
  required AuthoringWorkingHead candidateHead,
  required String candidateProjectJson,
  required int revision,
  required String questId,
  required String scriptModuleId,
}) => <String, Object?>{
  'ok': true,
  'outcome': 'prepared_unpublished',
  'basis_head_json': basisHead.canonicalJson,
  'head_json': candidateHead.canonicalJson,
  'project_json': candidateProjectJson,
  'revision': revision,
  'quest_id': questId,
  'script_module_id': scriptModuleId,
  'artifact_deduplicated': false,
  'build_status': 'blocked',
  'runtime_status': 'runtime_unqualified',
  'artifact_authority': 'not_granted',
  'source_inspection': 'fresh_capability_required',
  'publication_status': 'not_supported',
};

AuthoringRevision3QuestDraftIntentV3 _questIntent(int ordinal) =>
    AuthoringRevision3QuestDraftIntentV3(
      moduleNamespace: 'GoreMods.Quests.Managed$ordinal',
      technicalId: 'GORE_MANAGED_QUEST_$ordinal',
      textHelper: 'GoreManagedQuest${ordinal}Text',
      parentCatalogId: 'g1r:quest-parent:swampcamp_scchapter2',
      giverCatalogId: 'g1r:npc:om_grd_asghan_263',
      title: 'Managed Quest $ordinal',
      description: 'Exercise safe managed Quest publication.',
      objectiveTitle: 'Finish Managed Quest $ordinal',
    );

Map<String, Object?> _preparedResponse(AuthoringWorkingHead head) =>
    <String, Object?>{'ok': true, 'head_json': head.canonicalJson};

Map<String, Object?> _openedResponse(
  AuthoringWorkingHead head,
  String projectJson,
) => <String, Object?>{..._preparedResponse(head), 'project_json': projectJson};
