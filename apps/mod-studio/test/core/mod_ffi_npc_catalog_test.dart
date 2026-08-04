import 'dart:convert';
import 'dart:typed_data';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

const _gameRoot = r'C:\Games\Gothic Remake';
const _command = 'authoring_npc_archetype_catalog_v1_build_for_game_root';
const _bindingDomain =
    'gore-ffi.authoring-npc-archetype-catalog-v1.build-for-game-root.request-binding\u0000';

void main() {
  test(
    'static-linkage DTO is immutable and exposes no clone/build/runtime authorization',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{_command: _validResponse()},
      );

      final result = await ModFfi(
        core,
      ).authoringNpcArchetypeCatalogV1BuildForGameRoot(gameRoot: _gameRoot);

      expect(core.calls.single.command, _command);
      expect(core.calls.single.payload, <String, Object?>{
        'game_root': _gameRoot,
      });
      expect(result.requestBindingSha256, _requestBinding(_gameRoot));
      expect(result.generation.edition, 'g1r-steam');
      expect(result.recordCount, 1);
      expect(result.rejectionCount, 1);
      expect(result.records.single.spawn.className, 'USpawn_A');
      expect(result.records.single.aiConfig.className, 'UAi_A');
      expect(
        result.records.single.blueprintFamily,
        AuthoringNpcCatalogBlueprintFamily.humanBase,
      );
      expect(
        result.qualification.linkage,
        AuthoringNpcCatalogLinkageQualification.sealedLinkageVerified,
      );
      expect(
        result.qualification.runtime,
        AuthoringNpcCatalogRuntimeQualification.runtimeUnqualified,
      );
      expect(
        result.qualification.build,
        AuthoringNpcCatalogSupportStatus.notSupported,
      );
      expect(
        () => result.records.add(result.records.single),
        throwsUnsupportedError,
      );
      expect(() => result.rejections.clear(), throwsUnsupportedError);
    },
  );

  test('NPC catalog rejects outer confusion and loose raw JSON', () async {
    final mutations = <void Function(Map<String, Object?>)>[
      (response) => response['extra'] = true,
      (response) => response['ok'] = 1,
      (response) => response['request_binding_sha256'] = _hex('a'),
      (response) => response['request_binding_sha256'] = _requestBinding(
        _gameRoot,
      ).toUpperCase(),
      (response) => response['record_count'] = 2,
      (response) => response['rejection_count'] = 2,
      (response) =>
          (response['generation'] as Map<String, Object?>)['edition'] = 'other',
      (response) =>
          (response['qualification'] as Map<String, Object?>)['build'] =
              'supported',
      (response) =>
          ((response['source'] as Map<String, Object?>)['shipping_cache']
                  as Map<String, Object?>)['byte_len'] =
              999,
      (response) =>
          (response['payload_seal'] as Map<String, Object?>)['sha256'] = _hex(
            'b',
          ),
      (response) =>
          (response['catalog_seal'] as Map<String, Object?>)['sha256'] = _hex(
            'c',
          ),
      (response) =>
          response['catalog_json'] = ' ${(response['catalog_json'] as String)}',
      (response) => response['catalog_json'] =
          (response['catalog_json'] as String).replaceFirst(
            '"format":"npc_archetype_catalog"',
            '"format":"npc_archetype_catalog",'
                '"format":"npc_archetype_catalog"',
          ),
      (response) => response['catalog_json'] = '[]',
    ];

    for (final mutate in mutations) {
      final response = _validResponse();
      mutate(response);
      await expectLater(
        _call(response),
        throwsA(anyOf(isA<FormatException>(), isA<ModFfiException>())),
      );
    }
  });

  test('NPC catalog rejects resealed semantic tampering', () async {
    final mutations = <void Function(Map<String, Object?>)>[
      (artifact) => artifact['format'] = 'other',
      (artifact) => artifact['schema_revision'] = 2,
      (artifact) =>
          (artifact['catalog'] as Map<String, Object?>)['extra'] = true,
      (artifact) =>
          (((artifact['catalog'] as Map<String, Object?>)['qualification']
                  as Map<String, Object?>))['runtime'] =
              'runtime_qualified',
      (artifact) =>
          (_firstRecord(artifact)['spawn_ai_edge']
                  as Map<String, Object?>)['assigned_value'] =
              'UAi_OTHER',
      (artifact) => _firstRecord(artifact)['blueprint_family'] = 'unknown',
      (artifact) => _firstRecord(artifact)['evidence_sha256'] = _hex('0'),
      (artifact) =>
          (((_firstRecord(artifact)['spawn']
                  as Map<String, Object?>)['source_seal']
              as Map<String, Object?>))['sha256'] = _hex(
            '0',
          ),
      (artifact) =>
          (((_firstRecord(artifact)['spawn_ai_edge']
                  as Map<String, Object?>)['init_defaults_bytecode_seal']
              as Map<String, Object?>))['sha256'] = _hex(
            '0',
          ),
      (artifact) => _firstRecord(artifact)['extra'] = true,
      (artifact) => _firstRecord(artifact)['actor_blueprint'] = 'x' * 1025,
      (artifact) {
        final records = _payload(artifact)['records'] as List<Object?>;
        records.add(_deepCopy(records.single!));
      },
      (artifact) =>
          (((_payload(artifact)['rejections'] as List<Object?>).single
                      as Map<String, Object?>)['reason']
                  as Map<String, Object?>)['kind'] =
              'unknown',
      (artifact) {
        final catalog = artifact['catalog'] as Map<String, Object?>;
        final source = catalog['source'] as Map<String, Object?>;
        (source['shipping_cache'] as Map<String, Object?>)['byte_len'] = 999;
      },
    ];

    for (final mutate in mutations) {
      final response = _validResponse();
      _mutateAndReseal(response, mutate);
      await expectLater(_call(response), throwsFormatException);
    }
  });

  test(
    'NPC catalog order is native UTF-8 byte order, including non-BMP',
    () async {
      final utf8Sorted = _validResponse();
      _mutateAndReseal(utf8Sorted, (artifact) {
        _payload(artifact)['records'] = <Object?>[
          _record('\uE000'),
          _record('\u{10000}'),
        ];
        _payload(artifact)['rejections'] = <Object?>[
          _rejection('\uE000'),
          _rejection('\u{10000}'),
        ];
      });
      final accepted = await _call(utf8Sorted);
      expect(accepted.recordCount, 2);
      expect(accepted.rejectionCount, 2);

      final utf16OnlyOrder = _validResponse();
      _mutateAndReseal(utf16OnlyOrder, (artifact) {
        _payload(artifact)['records'] = <Object?>[
          _record('\u{10000}'),
          _record('\uE000'),
        ];
      });
      await expectLater(_call(utf16OnlyOrder), throwsFormatException);

      final reversedRejections = _validResponse();
      _mutateAndReseal(reversedRejections, (artifact) {
        _payload(artifact)['rejections'] = <Object?>[
          _rejection('\u{10000}'),
          _rejection('\uE000'),
        ];
      });
      await expectLater(_call(reversedRejections), throwsFormatException);
    },
  );

  test(
    'NPC catalog edge offsets are bounded by aligned init-defaults bytecode',
    () async {
      final maximum = _validResponse();
      _mutateAndReseal(maximum, (artifact) {
        final edge = _spawnAiEdge(artifact);
        (edge['init_defaults_bytecode_seal']
                as Map<String, Object?>)['byte_len'] =
            (1024 * 1024) * Uint32List.bytesPerElement;
        edge['instruction_offset_dwords'] = (1024 * 1024) - 1;
      });
      final accepted = await _call(maximum);
      expect(
        accepted.records.single.spawnAiEdge.instructionOffsetDwords,
        (1024 * 1024) - 1,
      );

      final outsideStream = _validResponse();
      _mutateAndReseal(outsideStream, (artifact) {
        final edge = _spawnAiEdge(artifact);
        (edge['init_defaults_bytecode_seal']
                as Map<String, Object?>)['byte_len'] =
            4;
        edge['instruction_offset_dwords'] = 1;
      });
      await expectLater(_call(outsideStream), throwsFormatException);

      final misaligned = _validResponse();
      _mutateAndReseal(misaligned, (artifact) {
        final edge = _spawnAiEdge(artifact);
        (edge['init_defaults_bytecode_seal']
                as Map<String, Object?>)['byte_len'] =
            5;
        edge['instruction_offset_dwords'] = 0;
      });
      await expectLater(_call(misaligned), throwsFormatException);

      final oversized = _validResponse();
      _mutateAndReseal(oversized, (artifact) {
        final edge = _spawnAiEdge(artifact);
        (edge['init_defaults_bytecode_seal']
                as Map<String, Object?>)['byte_len'] =
            ((1024 * 1024) * Uint32List.bytesPerElement) +
            Uint32List.bytesPerElement;
        edge['instruction_offset_dwords'] = 0;
      });
      await expectLater(_call(oversized), throwsFormatException);
    },
  );

  test(
    'NPC catalog enforces record and root request limits before use',
    () async {
      final tooManyRecords = _validResponse();
      _mutateAndReseal(tooManyRecords, (artifact) {
        final records = _payload(artifact)['records'] as List<Object?>;
        final template = records.single;
        records
          ..clear()
          ..addAll(List<Object?>.filled(4097, template));
      });
      await expectLater(_call(tooManyRecords), throwsFormatException);

      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{_command: _validResponse()},
      );
      for (final root in <String>[
        '',
        'bad\u0000root',
        'x' * (32 * 1024 + 1),
        String.fromCharCode(0xd800),
      ]) {
        await expectLater(
          ModFfi(
            core,
          ).authoringNpcArchetypeCatalogV1BuildForGameRoot(gameRoot: root),
          throwsArgumentError,
        );
      }
      expect(core.calls, isEmpty);
    },
  );
}

Future<AuthoringNpcArchetypeCatalogBuildResult> _call(
  Map<String, Object?> response,
) => ModFfi(
  FakeGoreCoreFfiService(
    responses: <String, Map<String, Object?>>{_command: response},
  ),
).authoringNpcArchetypeCatalogV1BuildForGameRoot(gameRoot: _gameRoot);

Map<String, Object?> _validResponse() {
  final generation = <String, Object?>{
    'edition': 'g1r-steam',
    'executable': <String, Object?>{
      'byte_len': 171698176,
      'sha256':
          'f406f969d3e73b6e58ea6e7aa10df7380318d97e7974d3be6e5a01183a4524f5',
    },
    'shipping_cache': <String, Object?>{
      'byte_len': 123394250,
      'sha256':
          '1018f1cfe6b99a650eecb33afb96752d691d2088ead27808971b812f04ecb4c2',
    },
    'binds_cache': <String, Object?>{
      'byte_len': 5903938,
      'sha256':
          '46e6629ad5cacc112b9922d48a1aa948f40572d7285705b981c3eca3dc615fea',
    },
  };
  final sourceIdentity = <String, Object?>{
    'shipping_cache': _deepCopy(generation['shipping_cache']!),
    'binds_cache': _deepCopy(generation['binds_cache']!),
  };
  final source = <String, Object?>{
    ...sourceIdentity,
    'source_pair_seal': _sealJson(jsonEncode(sourceIdentity)),
  };
  final payload = <String, Object?>{
    'extractor_records_sha256': _hex('4'),
    'records': <Object?>[_record('A')],
    'rejections': <Object?>[_rejection('Rejected')],
  };
  final catalog = <String, Object?>{
    'generation': generation,
    'story_catalog_seal': _fixedSeal('5', 500),
    'qualification': _qualification(),
    'source': source,
    'payload': payload,
    'payload_seal': _sealJson(jsonEncode(payload)),
  };
  final artifact = <String, Object?>{
    'format': 'npc_archetype_catalog',
    'schema_revision': 1,
    'catalog': catalog,
    'catalog_seal': _sealJson(jsonEncode(catalog)),
  };
  return <String, Object?>{
    'ok': true,
    'request_binding_sha256': _requestBinding(_gameRoot),
    'catalog_json': jsonEncode(artifact),
    'generation': _deepCopy(generation),
    'catalog_seal': _deepCopy(artifact['catalog_seal']!),
    'source': _deepCopy(source),
    'payload_seal': _deepCopy(catalog['payload_seal']!),
    'record_count': 1,
    'rejection_count': 1,
    'qualification': _deepCopy(catalog['qualification']!),
  };
}

Map<String, Object?> _record(String suffix) {
  final spawn = 'USpawn_$suffix';
  final ai = 'UAi_$suffix';
  final character = 'UCharacter_$suffix';
  const actor = 'BlueprintActor';
  return <String, Object?>{
    'spawn': _classEvidence(spawn, 'USpawn'),
    'ai_config': _classEvidence(ai, 'UAi'),
    'character_definition': _classEvidence(character, 'UCharacter'),
    'actor_blueprint': actor,
    'blueprint_family': 'human_base',
    'spawn_ai_edge': _edge(spawn, 'AIAgentConfigClass', ai, '6'),
    'spawn_blueprint_edge': _edge(spawn, 'AIAgentCharacterClass', actor, '7'),
    'ai_character_edge': _edge(ai, 'm_CharacterDefinition', character, '8'),
    'evidence_sha256': _hex('9'),
  };
}

Map<String, Object?> _rejection(String suffix) => <String, Object?>{
  'spawn_class': 'USpawn_$suffix',
  'reason': <String, Object?>{
    'kind': 'missing_init_defaults',
    'owner_class': 'USpawn_$suffix',
  },
};

Map<String, Object?> _classEvidence(String name, String superClass) =>
    <String, Object?>{
      'class_name': name,
      'super_class': superClass,
      'module_name': 'World',
      'relative_path': 'World/$name.as',
      'source_seal': _fixedSeal('a', 10),
    };

Map<String, Object?> _edge(
  String owner,
  String field,
  String value,
  String digest,
) => <String, Object?>{
  'owner_class': owner,
  'field_name': field,
  'assigned_value': value,
  'instruction_offset_dwords': 1,
  'init_defaults_bytecode_seal': _fixedSeal(digest, 20),
  'evidence_sha256': _hex(digest),
};

Map<String, Object?> _qualification() => <String, Object?>{
  'linkage': 'sealed_linkage_verified',
  'runtime': 'runtime_unqualified',
  'build': 'not_supported',
  'deploy': 'not_supported',
  'publication': 'not_supported',
};

Map<String, Object?> _fixedSeal(String digit, int byteLength) =>
    <String, Object?>{'byte_len': byteLength, 'sha256': _hex(digit)};

Map<String, Object?> _sealJson(String value) {
  final bytes = utf8.encode(value);
  return <String, Object?>{
    'byte_len': bytes.length,
    'sha256': crypto.sha256.convert(bytes).toString(),
  };
}

String _hex(String digit) => List<String>.filled(64, digit).join();

String _requestBinding(String root) {
  final bytes = utf8.encode(root);
  final length = Uint8List(8);
  ByteData.sublistView(length).setUint64(0, bytes.length, Endian.little);
  return crypto.sha256.convert(<int>[
    ...utf8.encode(_bindingDomain),
    ...length,
    ...bytes,
  ]).toString();
}

Map<String, Object?> _payload(Map<String, Object?> artifact) =>
    ((artifact['catalog'] as Map<String, Object?>)['payload']
        as Map<String, Object?>);

Map<String, Object?> _firstRecord(Map<String, Object?> artifact) =>
    ((_payload(artifact)['records'] as List<Object?>).first
        as Map<String, Object?>);

Map<String, Object?> _spawnAiEdge(Map<String, Object?> artifact) =>
    (_firstRecord(artifact)['spawn_ai_edge'] as Map<String, Object?>);

void _mutateAndReseal(
  Map<String, Object?> response,
  void Function(Map<String, Object?> artifact) mutate,
) {
  final artifact =
      jsonDecode(response['catalog_json']! as String) as Map<String, Object?>;
  mutate(artifact);
  final catalog = artifact['catalog'] as Map<String, Object?>;
  final source = catalog['source'] as Map<String, Object?>;
  final sourceIdentity = <String, Object?>{
    'shipping_cache': source['shipping_cache'],
    'binds_cache': source['binds_cache'],
  };
  source['source_pair_seal'] = _sealJson(jsonEncode(sourceIdentity));
  final payload = catalog['payload'] as Map<String, Object?>;
  catalog['payload_seal'] = _sealJson(jsonEncode(payload));
  artifact['catalog_seal'] = _sealJson(jsonEncode(catalog));

  response
    ..['catalog_json'] = jsonEncode(artifact)
    ..['generation'] = _deepCopy(catalog['generation']!)
    ..['catalog_seal'] = _deepCopy(artifact['catalog_seal']!)
    ..['source'] = _deepCopy(source)
    ..['payload_seal'] = _deepCopy(catalog['payload_seal']!)
    ..['record_count'] = (payload['records'] as List<Object?>).length
    ..['rejection_count'] = (payload['rejections'] as List<Object?>).length
    ..['qualification'] = _deepCopy(catalog['qualification']!);
}

Object _deepCopy(Object value) => jsonDecode(jsonEncode(value))!;
