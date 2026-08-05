import 'dart:convert';
import 'dart:typed_data';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_project_bootstrap.dart';

const _projectId = '0123456789abcdef0123456789abcdef';
const _v1ExecutableSha256 =
    'f406f969d3e73b6e58ea6e7aa10df7380318d97e7974d3be6e5a01183a4524f5';
const _v1ShippingSha256 =
    '1018f1cfe6b99a650eecb33afb96752d691d2088ead27808971b812f04ecb4c2';
const _v2ExecutableSha256 =
    'b52cd0453ad03987b833f7f26d09a2075109f18d653b8d4ff95271c857139e5d';
const _v2ShippingSha256 =
    '757d8624f0c7480f63cc14a1ba2d7e43f461a529064b0c0cfbf523a54639e385';
const _v1V2BindsSha256 =
    '46e6629ad5cacc112b9922d48a1aa948f40572d7285705b981c3eca3dc615fea';
const _v3ExecutableSha256 =
    'ab2c8d9e286a437bc5343748faf40959a77e9dc7c542ff9361f1ffaeca5c811c';
const _v3ShippingSha256 =
    '36124f1cdd4caae555423581aa40631af0ac80d5cef42528382739f932b0e728';
const _v3BindsSha256 =
    '854f58a695d0170144957f085c1e8c0f9ef40b271e35e90f79ffbccff8d999c5';

void main() {
  late AuthoringStoryCatalogGeneration generation;

  setUpAll(() async {
    generation = await _trustedGeneration();
  });

  test('creates exact canonical empty revision-3 bytes and exposed facts', () {
    final bootstrap = Revision3ProjectBootstrap.create(
      generation: generation,
      projectId: _projectId,
      name: 'My Story Mod',
      version: '1.2.3',
      author: 'Daniel',
      authoringLocales: const <String>['zh-Hans', 'de', 'en-US'],
    );

    expect(
      bootstrap.canonicalProjectJson,
      '{"format":2,"schema_revision":3,'
      '"project_id":"$_projectId","revision":0,'
      '"meta":{"name":"My Story Mod","version":"1.2.3",'
      '"author":"Daniel"},'
      '"target":{"executable":{"byte_len":171704320,'
      '"sha256":"$_v2ExecutableSha256"}},'
      '"authoring_locales":["de","en-US","zh-Hans"],'
      '"entities":{},"asset_store":{"assets":{}}}',
    );
    expect(bootstrap.identity.projectId, _projectId);
    expect(bootstrap.identity.revision, 0);
    expect(bootstrap.identity.name, 'My Story Mod');
    expect(bootstrap.identity.version, '1.2.3');
    expect(bootstrap.identity.author, 'Daniel');
    expect(bootstrap.identity.authoringLocales, const <String>[
      'de',
      'en-US',
      'zh-Hans',
    ]);
    expect(
      () => bootstrap.identity.authoringLocales.add('fr'),
      throwsUnsupportedError,
    );
    expect(bootstrap.target.edition, 'g1r-steam');
    expect(bootstrap.target.executableByteLength, 171704320);
    expect(bootstrap.target.executableSha256, _v2ExecutableSha256);
  });

  test('same set of locales always produces byte-identical JSON', () {
    final first = _create(
      generation,
      locales: const <String>['sl-rozaj-biske-1994', 'de', 'en-US'],
    );
    final second = _create(
      generation,
      locales: const <String>['en-US', 'sl-rozaj-biske-1994', 'de'],
    );

    expect(first.canonicalProjectJson, second.canonicalProjectJson);
    expect(first.identity.authoringLocales, const <String>[
      'de',
      'en-US',
      'sl-rozaj-biske-1994',
    ]);
    expect(
      jsonEncode(jsonDecode(first.canonicalProjectJson)),
      first.canonicalProjectJson,
    );
  });

  test('rejects non-canonical or zero caller ProjectIds', () {
    for (final projectId in <String>[
      '00000000000000000000000000000000',
      '0123456789ABCDEF0123456789ABCDEF',
      '0123456789abcdef0123456789abcde',
      '0123456789abcdef0123456789abcdef0',
      'g123456789abcdef0123456789abcdef',
      '0123456789abcdef0123456789abcdef\n',
    ]) {
      expect(
        () => _create(generation, projectId: projectId),
        throwsFormatException,
        reason: projectId,
      );
    }
  });

  test('requires trimmed non-empty bounded visible metadata', () {
    for (final name in <String>['', ' name', 'name ', 'bad\nname']) {
      expect(
        () => _create(generation, name: name),
        throwsFormatException,
        reason: 'name=$name',
      );
    }
    for (final version in <String>['', ' 1.0', '1.0 ', '1\t0']) {
      expect(
        () => _create(generation, version: version),
        throwsFormatException,
        reason: 'version=$version',
      );
    }
    for (final author in <String>[
      '',
      ' author',
      'author ',
      'bad\u007fauthor',
    ]) {
      expect(
        () => _create(generation, author: author),
        throwsFormatException,
        reason: 'author=$author',
      );
    }

    final exactUtf8Limit = _repeat('é', 128);
    expect(
      _create(generation, name: exactUtf8Limit).identity.name,
      exactUtf8Limit,
    );
    expect(
      () => _create(generation, name: _repeat('é', 129)),
      throwsFormatException,
    );
    expect(
      () => _create(generation, version: _repeat('x', 129)),
      throwsFormatException,
    );
    expect(
      () => _create(generation, author: _repeat('x', 257)),
      throwsFormatException,
    );
    expect(
      () => _create(generation, name: String.fromCharCode(0xd800)),
      throwsFormatException,
    );
  });

  test('mirrors Rust canonical BCP-47 shape and rejects duplicates', () {
    final valid = _create(
      generation,
      locales: const <String>['zh-Hans', 'sl-rozaj-biske-1994', 'en-US', 'de'],
    );
    expect(valid.identity.authoringLocales, const <String>[
      'de',
      'en-US',
      'sl-rozaj-biske-1994',
      'zh-Hans',
    ]);

    for (final locale in <String>[
      '',
      'e',
      'DE',
      'en-us',
      'zh-hans',
      'de-',
      'de--x',
      'de_foo',
      'de-abcdefghi',
      'dé',
      'abcdefgh-abcdefgh-abcdefgh-abcdefghi',
    ]) {
      expect(
        () => _create(generation, locales: <String>[locale]),
        throwsFormatException,
        reason: locale,
      );
    }
    expect(
      () => _create(generation, locales: const <String>['de', 'de']),
      throwsFormatException,
    );
    expect(
      () => _create(
        generation,
        locales: <String>[for (var index = 0; index < 65; index++) 'de-$index'],
      ),
      throwsFormatException,
    );
  });

  test(
    'rejects an all-zero executable digest at the bootstrap boundary',
    () async {
      final zeroDigestGeneration = await _trustedGeneration(
        executableSha256: _repeat('0', 64),
      );

      expect(() => _create(zeroDigestGeneration), throwsFormatException);
    },
  );

  test('accepts the exact V1/V2/V3 triples and rejects hybrids', () async {
    final v1 = await _trustedGeneration(
      executableByteLength: 171698176,
      executableSha256: _v1ExecutableSha256,
      shippingCacheSha256: _v1ShippingSha256,
    );
    final v3 = await _trustedGeneration(
      executableByteLength: 171787776,
      executableSha256: _v3ExecutableSha256,
      shippingCacheByteLength: 124352336,
      shippingCacheSha256: _v3ShippingSha256,
      bindsCacheByteLength: 5908587,
      bindsCacheSha256: _v3BindsSha256,
    );
    expect(_create(v1).target.executableSha256, _v1ExecutableSha256);
    expect(_create(generation).target.executableSha256, _v2ExecutableSha256);
    expect(_create(v3).target.executableSha256, _v3ExecutableSha256);

    final hybrids = <AuthoringStoryCatalogGeneration>[
      await _trustedGeneration(executableByteLength: 171698176),
      await _trustedGeneration(shippingCacheSha256: _v1ShippingSha256),
      await _trustedGeneration(bindsCacheSha256: _repeat('9', 64)),
      await _trustedGeneration(
        executableByteLength: 171787776,
        executableSha256: _v3ExecutableSha256,
      ),
      await _trustedGeneration(
        shippingCacheByteLength: 124352336,
        shippingCacheSha256: _v3ShippingSha256,
      ),
      await _trustedGeneration(
        bindsCacheByteLength: 5908587,
        bindsCacheSha256: _v3BindsSha256,
      ),
    ];
    for (final hybrid in hybrids) {
      expect(
        () => _create(hybrid),
        throwsFormatException,
        reason: hybrid.executable.sha256,
      );
    }
  });
}

Revision3ProjectBootstrap _create(
  AuthoringStoryCatalogGeneration generation, {
  String projectId = _projectId,
  String name = 'Project',
  String version = '1.0.0',
  String author = 'tests',
  Iterable<String> locales = const <String>[],
}) => Revision3ProjectBootstrap.create(
  generation: generation,
  projectId: projectId,
  name: name,
  version: version,
  author: author,
  authoringLocales: locales,
);

Future<AuthoringStoryCatalogGeneration> _trustedGeneration({
  int executableByteLength = 171704320,
  String executableSha256 = _v2ExecutableSha256,
  int shippingCacheByteLength = 123394250,
  String shippingCacheSha256 = _v2ShippingSha256,
  int bindsCacheByteLength = 5903938,
  String bindsCacheSha256 = _v1V2BindsSha256,
}) async {
  const gameRoot = r'C:\Games\Gothic 1 Remake';
  final generation = <String, Object?>{
    'edition': 'g1r-steam',
    'executable': _seal(executableSha256, executableByteLength),
    'shipping_cache': _seal(shippingCacheSha256, shippingCacheByteLength),
    'binds_cache': _seal(bindsCacheSha256, bindsCacheByteLength),
  };
  final catalogSeal = _seal(_repeat('4', 64), 5611);
  final catalogJson = jsonEncode(<String, Object?>{
    'format': 'story_catalog',
    'schema_revision': 1,
    'catalog': <String, Object?>{
      'generation': generation,
      'record_set_id': 'bootstrap-test-v1',
      'record_set_seal': _seal(_repeat('5', 64), 128),
      'npcs': <Object?>[],
      'quest_parents': <Object?>[],
    },
    'catalog_seal': catalogSeal,
  });
  final response = <String, Object?>{
    'ok': true,
    'request_binding_sha256': _gameRootBinding(gameRoot),
    'catalog_json': catalogJson,
    'generation': generation,
    'catalog_seal': catalogSeal,
  };
  final result = await ModFfi(
    FakeGoreCoreFfiService(
      responses: <String, Map<String, Object?>>{
        'authoring_story_catalog_v1_build_for_game_root': response,
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
