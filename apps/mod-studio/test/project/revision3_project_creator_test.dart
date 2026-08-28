import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/current_project_controller.dart';
import 'package:path/path.dart' as p;

const _projectId = '12121212121212121212121212121212';
const _executableSha256 =
    'b52cd0453ad03987b833f7f26d09a2075109f18d653b8d4ff95271c857139e5d';
const _shippingSha256 =
    '757d8624f0c7480f63cc14a1ba2d7e43f461a529064b0c0cfbf523a54639e385';
const _bindsSha256 =
    '46e6629ad5cacc112b9922d48a1aa948f40572d7285705b981c3eca3dc615fea';
const _latestExecutableSha256 =
    '824fbc94f2ac7f45927a0754605666c37af862d66156a15f8bf6813759d9e8e0';
const _latestShippingSha256 =
    '7a18f954e32af30fc24ae3a66ea35d3b5cb98560c8f5083c7846fc9ce1d77511';
const _latestBindsSha256 =
    'aa73402c11d4007035a2df32c55e50086a6d9c5b6da8619cdfcb4df53f02cea2';

void main() {
  test(
    'production creator recovers an exact project published before failure',
    () async {
      final fixture = Directory.systemTemp.createTempSync(
        'gore_r3_creator_recover',
      );
      addTearDown(() => fixture.deleteSync(recursive: true));
      final gameRoot = Directory(p.join(fixture.path, 'game'))..createSync();
      final projectRoot = Directory(p.join(fixture.path, 'project'))
        ..createSync();
      final generation = await _trustedGeneration(
        gameRoot.path,
        latestGeneration: true,
      );
      String? preparedProjectJson;
      var recoveryCalls = 0;
      late final _CreationLease recovered;
      final container = _creatorContainer(
        generation: generation,
        create: ({required root, required projectJson}) async {
          preparedProjectJson = projectJson;
          throw StateError('injected failure after fixed-head publication');
        },
        open: (root) async {
          recoveryCalls++;
          recovered = _CreationLease(
            root: root,
            projectId: _projectId,
            projectRevision: 0,
            canonicalProjectJson: preparedProjectJson!,
          );
          return recovered;
        },
      );
      addTearDown(container.dispose);

      final created = await container.read(
        managedRevision3CurrentProjectCreatorProvider,
      )(_request(projectRoot, gameRoot));

      expect(created, same(recovered));
      expect(recoveryCalls, 1);
      expect(preparedProjectJson, isNotNull);
      expect(recovered.canonicalProjectJson, preparedProjectJson);
      expect(recovered.closeCalls, 0);
      final project = (jsonDecode(preparedProjectJson!) as Map)
          .cast<String, Object?>();
      final target = (project['target']! as Map).cast<String, Object?>();
      expect(target['executable'], <String, Object?>{
        'byte_len': 171792384,
        'sha256': _latestExecutableSha256,
      });
    },
  );

  test('mismatched created candidate is closed and never adopted', () async {
    final fixture = Directory.systemTemp.createTempSync(
      'gore_r3_creator_mismatch',
    );
    addTearDown(() => fixture.deleteSync(recursive: true));
    final gameRoot = Directory(p.join(fixture.path, 'game'))..createSync();
    final projectRoot = Directory(p.join(fixture.path, 'project'))
      ..createSync();
    final generation = await _trustedGeneration(gameRoot.path);
    final mismatched = _CreationLease(
      root: projectRoot,
      projectId: _projectId,
      projectRevision: 0,
      canonicalProjectJson: '{"foreign":true}',
    );
    var recoveryCalls = 0;
    final container = _creatorContainer(
      generation: generation,
      create: ({required root, required projectJson}) async => mismatched,
      open: (root) async {
        recoveryCalls++;
        throw StateError('recovery must not run for an opened mismatch');
      },
    );
    addTearDown(container.dispose);

    await expectLater(
      container.read(managedRevision3CurrentProjectCreatorProvider)(
        _request(projectRoot, gameRoot),
      ),
      throwsA(isA<ManagedRevision3ProjectCreationException>()),
    );
    expect(mismatched.closeCalls, 1);
    expect(recoveryCalls, 0);
  });

  test(
    'arbitrary nonempty and game-overlapping destinations fail before hashing',
    () async {
      final fixture = Directory.systemTemp.createTempSync(
        'gore_r3_creator_roots',
      );
      addTearDown(() => fixture.deleteSync(recursive: true));
      final gameRoot = Directory(p.join(fixture.path, 'game'))..createSync();
      final nonempty = Directory(p.join(fixture.path, 'nonempty'))
        ..createSync();
      File(p.join(nonempty.path, 'notes.txt')).writeAsStringSync('user data');
      final generation = await _trustedGeneration(gameRoot.path);
      var generationLoads = 0;
      final container = _creatorContainer(
        generation: generation,
        onGenerationLoad: () => generationLoads++,
        create: ({required root, required projectJson}) async =>
            throw StateError('session create must not run'),
        open: (root) async => throw StateError('open must not run'),
      );
      addTearDown(container.dispose);
      final creator = container.read(
        managedRevision3CurrentProjectCreatorProvider,
      );

      await expectLater(
        creator(_request(nonempty, gameRoot)),
        throwsA(isA<ManagedRevision3ProjectCreationException>()),
      );
      await expectLater(
        creator(_request(gameRoot, gameRoot)),
        throwsA(isA<ManagedRevision3ProjectCreationException>()),
      );
      expect(generationLoads, 0);
    },
  );

  test(
    'a lock-only scaffold is rejected before hashing or session creation',
    () async {
      final fixture = Directory.systemTemp.createTempSync(
        'gore_r3_creator_retry',
      );
      addTearDown(() => fixture.deleteSync(recursive: true));
      final gameRoot = Directory(p.join(fixture.path, 'game'))..createSync();
      final projectRoot = Directory(p.join(fixture.path, 'project'))
        ..createSync();
      final control = Directory(p.join(projectRoot.path, '.gore'))
        ..createSync();
      final lock = File(p.join(control.path, 'session.lock'))
        ..writeAsStringSync('{}\n');
      final originalBytes = lock.readAsBytesSync();
      final generation = await _trustedGeneration(gameRoot.path);
      var generationLoads = 0;
      var createCalls = 0;
      final container = _creatorContainer(
        generation: generation,
        onGenerationLoad: () => generationLoads++,
        create: ({required root, required projectJson}) async {
          createCalls++;
          throw StateError('session create must not run');
        },
        open: (root) async => throw StateError('recovery must not run'),
      );
      addTearDown(container.dispose);

      await expectLater(
        container.read(managedRevision3CurrentProjectCreatorProvider)(
          _request(projectRoot, gameRoot),
        ),
        throwsA(isA<ManagedRevision3ProjectCreationException>()),
      );

      expect(generationLoads, 0);
      expect(createCalls, 0);
      expect(lock.readAsBytesSync(), originalBytes);
    },
  );

  test(
    'a hard-linked lock scaffold is rejected without touching its target',
    () async {
      final fixture = Directory.systemTemp.createTempSync(
        'gore_r3_creator_hardlink',
      );
      addTearDown(() => fixture.deleteSync(recursive: true));
      final gameRoot = Directory(p.join(fixture.path, 'game'))..createSync();
      final projectRoot = Directory(p.join(fixture.path, 'project'))
        ..createSync();
      final control = Directory(p.join(projectRoot.path, '.gore'))
        ..createSync();
      final target = File(p.join(fixture.path, 'hardlink-target.bin'))
        ..writeAsBytesSync(<int>[0, 1, 2, 3, 254, 255]);
      final originalBytes = target.readAsBytesSync();
      final lockPath = p.join(control.path, 'session.lock');
      final link = await Process.run('cmd.exe', <String>[
        '/c',
        'mklink',
        '/H',
        lockPath,
        target.path,
      ]);
      expect(
        link.exitCode,
        0,
        reason: 'mklink failed: ${link.stdout}${link.stderr}',
      );
      final generation = await _trustedGeneration(gameRoot.path);
      var generationLoads = 0;
      var createCalls = 0;
      final container = _creatorContainer(
        generation: generation,
        onGenerationLoad: () => generationLoads++,
        create: ({required root, required projectJson}) async {
          createCalls++;
          throw StateError('session create must not run');
        },
        open: (root) async => throw StateError('recovery must not run'),
      );
      addTearDown(container.dispose);

      await expectLater(
        container.read(managedRevision3CurrentProjectCreatorProvider)(
          _request(projectRoot, gameRoot),
        ),
        throwsA(isA<ManagedRevision3ProjectCreationException>()),
      );

      expect(generationLoads, 0);
      expect(createCalls, 0);
      expect(target.readAsBytesSync(), originalBytes);
      expect(File(lockPath).readAsBytesSync(), originalBytes);
    },
    skip: !Platform.isWindows
        ? 'Windows hard links require the Windows filesystem API'
        : false,
  );

  test('secure project id provider emits canonical nonzero identifiers', () {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final createId = container.read(managedRevision3ProjectIdFactoryProvider);
    final ids = <String>{for (var index = 0; index < 64; index++) createId()};

    expect(ids, hasLength(64));
    expect(ids, everyElement(matches(RegExp(r'^[0-9a-f]{32}$'))));
    expect(ids, isNot(contains('00000000000000000000000000000000')));
  });
}

ProviderContainer _creatorContainer({
  required AuthoringStoryCatalogGeneration generation,
  required ManagedRevision3ProjectSessionCreator create,
  required ManagedRevision3CurrentProjectOpener open,
  void Function()? onGenerationLoad,
}) => ProviderContainer(
  overrides: [
    managedRevision3StoryGenerationLoaderProvider.overrideWithValue((_) async {
      onGenerationLoad?.call();
      return generation;
    }),
    managedRevision3ProjectSessionCreatorProvider.overrideWithValue(create),
    managedRevision3CurrentProjectOpenerProvider.overrideWithValue(open),
    managedRevision3ProjectIdFactoryProvider.overrideWithValue(
      () => _projectId,
    ),
  ],
);

ManagedRevision3ProjectCreateRequest _request(
  Directory root,
  Directory gameRoot,
) => ManagedRevision3ProjectCreateRequest(
  root: root,
  gameRoot: gameRoot.path,
  name: 'Creator test',
  version: '0.1.0',
  author: 'Tests',
  authoringLocales: const <String>['de', 'en-US'],
);

final class _CreationLease implements ManagedRevision3CurrentProjectLease {
  _CreationLease({
    required this.root,
    required this.projectId,
    required this.projectRevision,
    required this.canonicalProjectJson,
  });

  @override
  final Directory root;
  @override
  final String projectId;
  @override
  final int projectRevision;
  @override
  String canonicalProjectJson;
  @override
  bool get requiresReopen => false;
  @override
  AuthoringWorkingHead get head => AuthoringWorkingHead.fromCanonicalJson(
    '{"store_format":1,"snapshot":{"byte_len":1,'
    '"sha256":"${_repeat('1', 64)}"}}',
  );

  int closeCalls = 0;

  @override
  Future<void> close() async => closeCalls++;

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

Future<AuthoringStoryCatalogGeneration> _trustedGeneration(
  String gameRoot, {
  bool latestGeneration = false,
}) async {
  final executableSha256 = latestGeneration
      ? _latestExecutableSha256
      : _executableSha256;
  final shippingSha256 = latestGeneration
      ? _latestShippingSha256
      : _shippingSha256;
  final bindsSha256 = latestGeneration ? _latestBindsSha256 : _bindsSha256;
  final generation = <String, Object?>{
    'edition': 'g1r-steam',
    'executable': _seal(
      executableSha256,
      latestGeneration ? 171792384 : 171704320,
    ),
    'shipping_cache': _seal(
      shippingSha256,
      latestGeneration ? 124459412 : 123394250,
    ),
    'binds_cache': _seal(bindsSha256, latestGeneration ? 5908985 : 5903938),
  };
  final catalogSeal = _seal(_repeat('4', 64), 5611);
  final catalogJson = jsonEncode(<String, Object?>{
    'format': 'story_catalog',
    'schema_revision': 1,
    'catalog': <String, Object?>{
      'generation': generation,
      'record_set_id': latestGeneration ? 'creator-test-v4' : 'creator-test-v2',
      'record_set_seal': _seal(_repeat('5', 64), 128),
      'npcs': <Object?>[],
      'quest_parents': <Object?>[],
    },
    'catalog_seal': catalogSeal,
  });
  final result = await ModFfi(
    FakeGoreCoreFfiService(
      responses: <String, Map<String, Object?>>{
        'authoring_story_catalog_v1_build_for_game_root': <String, Object?>{
          'ok': true,
          'request_binding_sha256': _gameRootBinding(gameRoot),
          'catalog_json': catalogJson,
          'generation': generation,
          'catalog_seal': catalogSeal,
        },
      },
    ),
  ).authoringStoryCatalogV1BuildForGameRoot(gameRoot: gameRoot);
  return result.generation;
}

Map<String, Object?> _seal(String sha256, int byteLength) => <String, Object?>{
  'byte_len': byteLength,
  'sha256': sha256,
};

String _gameRootBinding(String gameRoot) {
  final encoded = utf8.encode(gameRoot);
  final length = Uint8List(8);
  ByteData.sublistView(length).setUint64(0, encoded.length, Endian.little);
  return crypto.sha256.convert(<int>[
    ...utf8.encode(
      'gore-story-catalog.authoring-build-for-game-root-v1.request-binding\u0000',
    ),
    ...length,
    ...encoded,
  ]).toString();
}

String _repeat(String value, int count) =>
    List<String>.filled(count, value).join();
