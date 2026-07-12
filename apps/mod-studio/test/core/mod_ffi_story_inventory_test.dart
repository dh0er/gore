import 'dart:convert';
import 'dart:typed_data';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

const _executable = 'A/game.exe';
const _shipping = 'B/Shipping.cache';
const _binds = 'C/Binds.cache';
const _bindingDomain =
    'gore-story-inventory.authoring-build-v1.request-binding\u0000';

void main() {
  test('Story inventory build returns one strictly bound closed DTO', () async {
    expect(
      _requestBinding(),
      'fb5cadf2935156f4df183cf86321325de63dd3004f90a92603b801ee6fdd3c1c',
    );
    final response = _validResponse();
    final core = FakeGoreCoreFfiService(
      responses: {'authoring_story_inventory_v1_build': response},
    );

    final result = await ModFfi(core).authoringStoryInventoryV1Build(
      executable: _executable,
      shippingCache: _shipping,
      bindsCache: _binds,
    );

    expect(result.requestBindingSha256, _requestBinding());
    expect(result.generation.edition, 'g1r-steam');
    expect(result.catalogLayer, 'base-game.g1r.scripts.inventory.v1');
    expect(result.coverage, AuthoringStoryInventoryCoverage.baseGameOnly);
    expect(
      result.runtimeQualification,
      AuthoringStoryInventoryRuntimeQualification.runtimeUnqualified,
    );
    expect(
      result.publicationStatus,
      AuthoringStoryInventoryPublicationStatus.notSupported,
    );
    expect(result.modules, ['base.alpha', 'base.zeta']);
    expect(result.relativePaths, ['base/alpha.as', 'base/zeta.as']);
    expect(result.symbols, ['nativecall', 'tailonlytype']);
    expect(() => result.modules.add('forged'), throwsUnsupportedError);
    expect(result.inventoryJson, isNot(contains(_executable)));
    expect(core.calls.single.command, 'authoring_story_inventory_v1_build');
    expect(core.calls.single.payload, <String, Object?>{
      'executable': _executable,
      'shipping_cache': _shipping,
      'binds_cache': _binds,
    });
  });

  test(
    'Story inventory rejects confusion, tampering, and loose data',
    () async {
      final malformed = <Map<String, Object?> Function()>[
        () => _validResponse()..['extra'] = true,
        () =>
            _validResponse()
              ..['request_binding_sha256'] = List.filled(64, 'f').join(),
        () {
          final response = _validResponse();
          response['inventory_json'] = '${response['inventory_json']}\n';
          return response;
        },
        () {
          final response = _validResponse();
          response['inventory_json'] = (response['inventory_json'] as String)
              .replaceFirst(
                '"format":"story_script_collision_inventory"',
                '"format":"story_script_collision_inventory",'
                    '"format":"story_script_collision_inventory"',
              );
          return response;
        },
        () => _rewriteArtifact((payload, artifact) => payload['extra'] = true),
        () => _rewriteArtifact(
          (payload, artifact) => artifact['schema_revision'] = 1.0,
        ),
        () => _rewriteArtifact(
          (payload, artifact) =>
              (payload['generation'] as Map<String, Object?>)['edition'] =
                  'g1r-other',
        ),
        () => _rewriteArtifact(
          (payload, artifact) => payload['story_catalog_seal'] = _seal('f', 7),
        ),
        () => _rewriteArtifact(
          (payload, artifact) =>
              (payload['source'] as Map<String, Object?>)['shipping_cache'] =
                  _seal('f', 10),
        ),
        () => _rewriteArtifact(
          (payload, artifact) =>
              (payload['source'] as Map<String, Object?>)['source_pair_seal'] =
                  _seal('f', 15),
        ),
        () => _rewriteArtifact(
          (payload, artifact) => payload['coverage'] = 'resolved_loadout',
          syncOuterCapabilities: true,
        ),
        () => _rewriteArtifact(
          (payload, artifact) =>
              payload['runtime_qualification'] = 'runtime_qualified',
          syncOuterCapabilities: true,
        ),
        () => _rewriteArtifact(
          (payload, artifact) => payload['publication_status'] = 'supported',
          syncOuterCapabilities: true,
        ),
        () => _rewriteArtifact(
          (payload, artifact) =>
              payload['modules'] = ['base.zeta', 'base.alpha'],
        ),
        () => _rewriteArtifact(
          (payload, artifact) => payload['symbols'] = ['Uppercase'],
        ),
        () {
          final response = _validResponse();
          (response['payload_seal'] as Map<String, Object?>)['sha256'] =
              List.filled(64, 'e').join();
          return response;
        },
        () {
          final response = _validResponse();
          (response['generation'] as Map<String, Object?>)['shipping_cache'] =
              _seal('e', 10);
          return response;
        },
        () =>
            _validResponse()..['catalog_layer'] = 'resolved-loadout.scripts.v1',
        () => _validResponse()..['coverage'] = 'resolved_loadout',
      ];

      for (final buildResponse in malformed) {
        final core = FakeGoreCoreFfiService(
          responses: {'authoring_story_inventory_v1_build': buildResponse()},
        );
        await expectLater(
          ModFfi(core).authoringStoryInventoryV1Build(
            executable: _executable,
            shippingCache: _shipping,
            bindsCache: _binds,
          ),
          throwsFormatException,
        );
      }
    },
  );

  test(
    'Story inventory bounds paths and response content before use',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: {'authoring_story_inventory_v1_build': _validResponse()},
      );
      final ffi = ModFfi(core);
      for (final executable in <String>[
        '',
        'bad\u0000path',
        List.filled(32 * 1024 + 1, 'x').join(),
        String.fromCharCode(0xd800),
      ]) {
        await expectLater(
          ffi.authoringStoryInventoryV1Build(
            executable: executable,
            shippingCache: _shipping,
            bindsCache: _binds,
          ),
          throwsArgumentError,
        );
      }
      expect(core.calls, isEmpty);

      final oversized = _validResponse()
        ..['inventory_json'] = List.filled(24 * 1024 * 1024 + 1, 'x').join();
      await expectLater(
        ModFfi(
          FakeGoreCoreFfiService(
            responses: {'authoring_story_inventory_v1_build': oversized},
          ),
        ).authoringStoryInventoryV1Build(
          executable: _executable,
          shippingCache: _shipping,
          bindsCache: _binds,
        ),
        throwsFormatException,
      );

      final longEntry = _rewriteArtifact(
        (payload, artifact) =>
            payload['symbols'] = [List.filled(513, 'x').join()],
      )..['payload_seal'] = 'seal parsing must not run';
      await expectLater(
        ModFfi(
          FakeGoreCoreFfiService(
            responses: {'authoring_story_inventory_v1_build': longEntry},
          ),
        ).authoringStoryInventoryV1Build(
          executable: _executable,
          shippingCache: _shipping,
          bindsCache: _binds,
        ),
        throwsA(
          isA<FormatException>().having(
            (error) => error.message.toString(),
            'message',
            contains('symbols entry 0 is invalid'),
          ),
        ),
      );

      final suffix = List.filled(507, 'x').join();
      final aggregateOversize = _rewriteArtifact(
        (payload, artifact) => payload['symbols'] = List.generate(
          32768,
          (index) => '${index.toString().padLeft(5, '0')}$suffix',
          growable: false,
        ),
      )..['payload_seal'] = 'seal parsing must not run';
      await expectLater(
        ModFfi(
          FakeGoreCoreFfiService(
            responses: {
              'authoring_story_inventory_v1_build': aggregateOversize,
            },
          ),
        ).authoringStoryInventoryV1Build(
          executable: _executable,
          shippingCache: _shipping,
          bindsCache: _binds,
        ),
        throwsA(
          isA<FormatException>().having(
            (error) => error.message.toString(),
            'message',
            contains('aggregate byte limit'),
          ),
        ),
      );
    },
  );
}

Map<String, Object?> _validResponse() {
  final generation = <String, Object?>{
    'edition': 'g1r-steam',
    'executable': _seal('1', 20),
    'shipping_cache': _seal('2', 10),
    'binds_cache': _seal('3', 5),
  };
  final storyCatalogSeal = _seal('4', 7);
  final sourcePairSeal = _seal('5', 15);
  final payload = <String, Object?>{
    'generation': generation,
    'story_catalog_seal': storyCatalogSeal,
    'catalog_layer': 'base-game.g1r.scripts.inventory.v1',
    'coverage': 'base_game_only',
    'runtime_qualification': 'runtime_unqualified',
    'publication_status': 'not_supported',
    'source': <String, Object?>{
      'shipping_cache': _seal('2', 10),
      'binds_cache': _seal('3', 5),
      'source_pair_seal': sourcePairSeal,
    },
    'modules': <Object?>['base.alpha', 'base.zeta'],
    'relative_paths': <Object?>['base/alpha.as', 'base/zeta.as'],
    'symbols': <Object?>['nativecall', 'tailonlytype'],
  };
  final payloadSeal = _payloadSeal(payload);
  final artifact = <String, Object?>{
    'format': 'story_script_collision_inventory',
    'schema_revision': 1,
    'inventory': payload,
    'payload_seal': payloadSeal,
  };
  return <String, Object?>{
    'ok': true,
    'request_binding_sha256': _requestBinding(),
    'inventory_json': jsonEncode(artifact),
    'generation': _deepCopy(generation),
    'story_catalog_seal': _deepCopy(storyCatalogSeal),
    'source_pair_seal': _deepCopy(sourcePairSeal),
    'payload_seal': _deepCopy(payloadSeal),
    'catalog_layer': 'base-game.g1r.scripts.inventory.v1',
    'coverage': 'base_game_only',
    'runtime_qualification': 'runtime_unqualified',
    'publication_status': 'not_supported',
  };
}

Map<String, Object?> _rewriteArtifact(
  void Function(Map<String, Object?> payload, Map<String, Object?> artifact)
  mutate, {
  bool syncOuterCapabilities = false,
}) {
  final response = _validResponse();
  final artifact = (jsonDecode(response['inventory_json'] as String) as Map)
      .cast<String, Object?>();
  final payload = (artifact['inventory'] as Map).cast<String, Object?>();
  mutate(payload, artifact);
  final payloadSeal = _payloadSeal(payload);
  artifact['payload_seal'] = payloadSeal;
  response['payload_seal'] = _deepCopy(payloadSeal);
  response['inventory_json'] = jsonEncode(artifact);
  if (syncOuterCapabilities) {
    response['coverage'] = payload['coverage'];
    response['runtime_qualification'] = payload['runtime_qualification'];
    response['publication_status'] = payload['publication_status'];
  }
  return response;
}

Map<String, Object?> _payloadSeal(Map<String, Object?> payload) {
  final bytes = utf8.encode(jsonEncode(payload));
  return <String, Object?>{
    'byte_len': bytes.length,
    'sha256': crypto.sha256.convert(bytes).toString(),
  };
}

Map<String, Object?> _seal(String byte, int length) => <String, Object?>{
  'byte_len': length,
  'sha256': List.filled(64, byte).join(),
};

Map<String, Object?> _deepCopy(Map<String, Object?> value) =>
    (jsonDecode(jsonEncode(value)) as Map).cast<String, Object?>();

String _requestBinding() {
  final bytes = <int>[...utf8.encode(_bindingDomain)];
  for (final value in <String>[_executable, _shipping, _binds]) {
    final encoded = utf8.encode(value);
    final length = Uint8List(8);
    ByteData.sublistView(length).setUint64(0, encoded.length, Endian.little);
    bytes
      ..addAll(length)
      ..addAll(encoded);
  }
  return crypto.sha256.convert(bytes).toString();
}
