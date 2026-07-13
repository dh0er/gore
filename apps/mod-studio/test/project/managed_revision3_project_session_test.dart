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

      expect(core.calls.map((call) => call.command), <String>[
        'authoring_store_open_revision3',
        'authoring_store_prepare_revision3_checkpoint',
        'authoring_store_open_revision3_head_bytes',
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

  test('derive callback cannot re-enter save, derive, or close', () async {
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
        session.close(),
        throwsA(isA<ManagedProjectReentrantOperationException>()),
      );
      return const ManagedProjectDerivedRejection<String>('closed');
    });
    expect(result, 'closed');
    expect(session.isClosed, isFalse);
    await session.close();
  });

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

final class _FakeRevision3Store implements ManagedRevision3AuthoringStore {
  final Map<String, String> _projectsByHead = <String, String>{};
  final List<AuthoringAssetVerification> openVerifications =
      <AuthoringAssetVerification>[];
  final List<AuthoringAssetVerification> headVerifications =
      <AuthoringAssetVerification>[];
  final List<String?> expectedHeads = <String?>[];
  int _sequence = 0;
  int prepareCalls = 0;
  _AfterPrepare? afterPrepare;
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

Map<String, Object?> _preparedResponse(AuthoringWorkingHead head) =>
    <String, Object?>{'ok': true, 'head_json': head.canonicalJson};

Map<String, Object?> _openedResponse(
  AuthoringWorkingHead head,
  String projectJson,
) => <String, Object?>{..._preparedResponse(head), 'project_json': projectJson};
