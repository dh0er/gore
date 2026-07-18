import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:path/path.dart' as p;

Map<String, Object?> _compiledScriptReport(String miniPath) =>
    <String, Object?>{
      'ok': true,
      'outcome': 'compiled',
      'mini_path': miniPath,
      'module': 'GoreMods.Probe',
      'compile_error': null,
      'compiler_diagnostics': <String, Object?>{
        'capture': 'captured',
        'messages': <Object?>[],
        'omitted': 0,
      },
      'install_restore': 'restored_exact',
      'recovery_required': false,
    };

const _storyCatalogRequest = '{"format":"story_catalog"}';
const _storyCatalogGameRoot = 'C:/Games/Gothic';
const _storyCatalogExecutable = r'C:\Game\Gothic1Remake.exe';
const _storyCatalogShippingCache =
    r'C:\Game\Alkimia\Content\Paks\Shipping-G1-Game.cache';
const _storyCatalogBindsCache =
    r'C:\Game\Alkimia\Content\Paks\Binds-G1-Game.cache';

Map<String, Object?> _catalogContentSeal(String byte, int byteLength) => {
  'byte_len': byteLength,
  'sha256': List.filled(64, byte).join(),
};

String _storyCatalogAlias(String catalogId, String role) {
  final bytes = <int>[
    ...utf8.encode('gore-story-catalog.authoring-selector-v1\u0000'),
  ];
  for (final value in <String>[catalogId, role]) {
    final encoded = utf8.encode(value);
    final length = Uint8List(8);
    ByteData.sublistView(length).setUint64(0, encoded.length, Endian.little);
    bytes
      ..addAll(length)
      ..addAll(encoded);
  }
  return 'Catalog_${crypto.sha256.convert(bytes)}';
}

Map<String, Object?> _storyCatalogClass(
  String catalogId,
  String role,
  String sealByte,
  String runtimeClass,
) => <String, Object?>{
  'catalog_layer': 'base-game.g1r.scripts',
  'authoring_selector': _storyCatalogAlias(catalogId, role),
  'source_catalog_selector': 'script-class:Trusted/$runtimeClass',
  'runtime_class': runtimeClass,
  'source_seal': _catalogContentSeal(sealByte, 100),
};

Map<String, Object?> _storyCatalogNpc({required bool viper}) {
  final runtime = viper ? 'OM_STT_Viper_302' : 'OM_GRD_Asghan_263';
  final catalogId = viper
      ? 'g1r:npc:om_stt_viper_302'
      : 'g1r:npc:om_grd_asghan_263';
  final character = _storyCatalogClass(
    catalogId,
    'character_definition',
    viper ? 'e' : 'a',
    'UCharacterDefinition_Human_$runtime',
  );
  return <String, Object?>{
    'catalog_id': catalogId,
    'display_name': viper ? 'Viper' : 'Asghan',
    'runtime_unique_name': runtime,
    'character_definition': character,
    'ai_agent_config': _storyCatalogClass(
      catalogId,
      'ai_agent_config',
      viper ? 'd' : 'b',
      'UAIAgentConfig_Human_$runtime',
    ),
    'spawn_definition': _storyCatalogClass(
      catalogId,
      'spawn_definition',
      'c',
      'USpawnAIAgentDefinition_$runtime',
    ),
    'quest_giver': <String, Object?>{
      'catalog_layer': character['catalog_layer'],
      'authoring_selector': _storyCatalogAlias(catalogId, 'quest_giver'),
      'source_catalog_selector': character['source_catalog_selector'],
      'runtime_unique_name': runtime,
      'source_seal': character['source_seal'],
    },
    'discovery_status': 'sealed_cache_defaults_verified',
    'authoring_qualification': 'offline_qualified',
    'runtime_qualification': 'runtime_unqualified',
    'evidence_id': viper
        ? 'npc-logical-clone-v1:viper-current-v1'
        : 'npc-logical-clone-v1',
    'blocks_build': true,
  };
}

Map<String, Object?> _validStoryCatalogResponse({
  String catalogJson = _storyCatalogRequest,
}) => <String, Object?>{
  'ok': true,
  'request_catalog_sha256': crypto.sha256
      .convert(utf8.encode(catalogJson))
      .toString(),
  'selections': <String, Object?>{
    'schema_revision': 1,
    'generation': <String, Object?>{
      'edition': 'g1r-steam',
      'executable': _catalogContentSeal('1', 171698176),
      'shipping_cache': _catalogContentSeal('2', 123394250),
      'binds_cache': _catalogContentSeal('3', 5903938),
    },
    'catalog_seal': _catalogContentSeal('4', 5611),
    'npcs': <Object?>[
      _storyCatalogNpc(viper: false),
      _storyCatalogNpc(viper: true),
    ],
    'quest_parents': <Object?>[
      <String, Object?>{
        'catalog_id': 'g1r:quest-parent:swampcamp_scchapter2',
        'display_name': 'Swamp Camp â€” Chapter 2',
        'quest_class': _storyCatalogClass(
          'g1r:quest-parent:swampcamp_scchapter2',
          'quest_parent',
          'f',
          'UQuest_SwampCamp_SCCHAPTER2',
        ),
        'parent_class_name': 'UQuest_SwampCamp',
        'role': 'chapter',
        'qualification': 'curated_defaults_verified',
        'transition_qualification': 'runtime_unqualified',
        'evidence_id': 'current-cache-defaults-swampcamp-chapter2-20260712',
        'blocks_build': true,
      },
    ],
    'quest_collision_catalog': <String, Object?>{
      'status': 'inventory_unavailable',
      'catalog_layer': 'resolved-loadout.scripts.v1',
      'source_seal': _catalogContentSeal('2', 123394250),
      'blocks_draft_creation': true,
    },
    'blocks_build': true,
  },
};

Map<String, Object?> _storyCatalogGeneration() => <String, Object?>{
  'edition': 'g1r-steam',
  'executable': _catalogContentSeal('1', 171698176),
  'shipping_cache': _catalogContentSeal('2', 123394250),
  'binds_cache': _catalogContentSeal('3', 5903938),
};

String _storyCatalogBuildRaw() => jsonEncode(<String, Object?>{
  'format': 'story_catalog',
  'schema_revision': 1,
  'catalog': <String, Object?>{
    'generation': _storyCatalogGeneration(),
    'record_set_id': 'g1r-steam-1.0.3-curated-story-v1',
    'record_set_seal': _catalogContentSeal('5', 5499),
    'npcs': <Object?>[],
    'quest_parents': <Object?>[],
  },
  'catalog_seal': _catalogContentSeal('4', 5611),
});

String _storyCatalogBuildBinding({
  String executable = _storyCatalogExecutable,
  String shippingCache = _storyCatalogShippingCache,
  String bindsCache = _storyCatalogBindsCache,
}) {
  final bytes = <int>[
    ...utf8.encode(
      'gore-story-catalog.authoring-build-v1.request-binding\u0000',
    ),
  ];
  for (final value in <String>[executable, shippingCache, bindsCache]) {
    final encoded = utf8.encode(value);
    final length = Uint8List(8);
    ByteData.sublistView(length).setUint64(0, encoded.length, Endian.little);
    bytes
      ..addAll(length)
      ..addAll(encoded);
  }
  return crypto.sha256.convert(bytes).toString();
}

Map<String, Object?> _validStoryCatalogBuildResponse({String? catalogJson}) =>
    <String, Object?>{
      'ok': true,
      'request_binding_sha256': _storyCatalogBuildBinding(),
      'catalog_json': catalogJson ?? _storyCatalogBuildRaw(),
      'generation': _storyCatalogGeneration(),
      'catalog_seal': _catalogContentSeal('4', 5611),
    };

String _storyCatalogGameRootBinding({String gameRoot = _storyCatalogGameRoot}) {
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

Map<String, Object?> _validStoryCatalogGameRootBuildResponse({
  String? catalogJson,
}) => <String, Object?>{
  'ok': true,
  'request_binding_sha256': _storyCatalogGameRootBinding(),
  'catalog_json': catalogJson ?? _storyCatalogBuildRaw(),
  'generation': _storyCatalogGeneration(),
  'catalog_seal': _catalogContentSeal('4', 5611),
};

Map<String, Object?> _validVoiceMatchResponse() => {
  'ok': true,
  'archive': r'C:\Game\VoiceOver\german.zip',
  'archive_size': 4096,
  'archive_sha256': List.filled(32, 'ab').join(),
  'loc_id': 'LINE_ONE',
  'expected_basename': 'LINE_ONE.ogg',
  'resolution': 'unique',
  'match_count': 1,
  'matches': <Object?>[
    <String, Object?>{
      'index': 7,
      'path': 'Voices/Hero/line_one.OGG',
      'basename': 'line_one.OGG',
      'compressed_size': 100,
      'uncompressed_size': 128,
      'crc32': 0x12345678,
      'compression': 'stored',
      'compression_code': 0,
      'last_modified': <String, Object?>{
        'year': 2026,
        'month': 7,
        'day': 12,
        'hour': 13,
        'minute': 14,
        'second': 16,
      },
      'unix_mode': 0x81a4,
      'is_directory': false,
      'is_symlink': false,
      'encrypted': false,
    },
  ],
};

Map<String, Object?> _validVoiceOggInspectResponse() => {
  'ok': true,
  'codec': 'vorbis',
  'pages': 2,
  'streams': 1,
  'content_seal': <String, Object?>{
    'byte_len': 4096,
    'sha256': List.filled(64, 'a').join(),
  },
};

Map<String, Object?> _firstVoiceMatch(Map<String, Object?> response) =>
    ((response['matches'] as List<Object?>).single as Map)
        .cast<String, Object?>();

Map<String, Object?> _voiceTimestamp(Map<String, Object?> response) =>
    (_firstVoiceMatch(response)['last_modified'] as Map)
        .cast<String, Object?>();

String _validWorkingHeadJson() =>
    '{"store_format":1,"snapshot":{"byte_len":321,"sha256":"${List.filled(64, 'a').join()}"}}';

Future<ModFfiException> _captureModFfiException(Future<Object?> call) async {
  try {
    await call;
  } on ModFfiException catch (error) {
    return error;
  }
  fail('expected ModFfiException');
}

class _MalformedJsonCoreService extends GoreCoreFfiService {
  @override
  String get description => 'malformed response fake';

  @override
  bool get isAvailable => true;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) => throw const FormatException('hostile undecodable native response');
}

void main() {
  test('normal success response is returned to the command wrapper', () async {
    final core = FakeGoreCoreFfiService(
      responses: {
        'find_game': {
          'ok': true,
          'found': true,
          'exe': r'C:\Game\GothicRemake.exe',
        },
      },
    );

    expect(await ModFfi(core).findGameExe(), r'C:\Game\GothicRemake.exe');
    expect(core.calls.single.command, 'find_game');
  });

  test('texture index accepts only one bounded canonical generation', () async {
    final core = FakeGoreCoreFfiService(
      responses: {
        'texture_index': {
          'ok': true,
          'build_id': 'build-1',
          'count': 2,
          'entries': {
            '/Game/Textures/T_A': '0',
            '/Engine/Textures/T_B': '18446744073709551615',
          },
        },
      },
    );

    final snapshot = await ModFfi(core).textureIndex(r'C:\Game');

    expect(snapshot.buildId, 'build-1');
    expect(snapshot.entries, {
      '/Game/Textures/T_A': '0',
      '/Engine/Textures/T_B': '18446744073709551615',
    });
  });

  test('texture index rejects malformed native boundaries', () async {
    final invalidResponses = <Map<String, Object?>>[
      {
        'ok': true,
        'build_id': 'build-1',
        'count': 1.0,
        'entries': {'/Game/Textures/T_A': '1'},
      },
      {
        'ok': true,
        'build_id': 'build-1',
        'count': 1,
        'entries': {'Game/Textures/T_A': '1'},
      },
      {
        'ok': true,
        'build_id': 'build-1',
        'count': 1,
        'entries': {'/Game/Textures/T_A': '01'},
      },
      {
        'ok': true,
        'build_id': 'build-1',
        'count': 1,
        'entries': {'/Game/Textures/T_A': '18446744073709551616'},
      },
      {
        'ok': true,
        'build_id': 'build-1',
        'count': 2,
        'entries': {'/Game/Textures/T_A': '1', '/game/Textures/T_A': '2'},
      },
    ];

    for (final response in invalidResponses) {
      final core = FakeGoreCoreFfiService(
        responses: {'texture_index': response},
      );
      await expectLater(
        ModFfi(core).textureIndex(r'C:\Game'),
        throwsFormatException,
      );
    }
  });

  test(
    'texture extract validates generation and exact indexed identity',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: {
          'texture_extract': {'ok': true},
        },
      );
      final ffi = ModFfi(core);

      await ffi.textureExtract(
        r'C:\Game',
        expectedBuildId: 'build-1',
        asset: '/Game/Textures/T_A',
        packageId: '42',
      );

      expect(core.calls.single.payload, {
        'game': r'C:\Game',
        'expected_build_id': 'build-1',
        'asset': '/Game/Textures/T_A',
        'package_id': '42',
      });
      await expectLater(
        ffi.textureExtract(
          r'C:\Game',
          expectedBuildId: 'build-1',
          asset: '/Game/Textures/T_A',
          packageId: '042',
        ),
        throwsArgumentError,
      );
      expect(core.calls, hasLength(1));
    },
  );

  test('texture preview capability wrappers bind token and offset', () async {
    final token = 'd' * 64;
    final core = FakeGoreCoreFfiService(
      responses: {
        'texture_preview_read': {'ok': true},
        'texture_preview_release': {'ok': true},
      },
    );
    final ffi = ModFfi(core);

    await ffi.texturePreviewRead(previewToken: token, offset: 17);
    await ffi.texturePreviewRelease(previewToken: token);

    expect(core.calls[0].payload, {'preview_token': token, 'offset': 17});
    expect(core.calls[1].payload, {'preview_token': token});
    expect(
      () => ffi.texturePreviewRead(previewToken: 'D' * 64, offset: 0),
      throwsArgumentError,
    );
    expect(
      () =>
          ffi.texturePreviewRead(previewToken: token, offset: 64 * 1024 * 1024),
      throwsArgumentError,
    );
    expect(core.calls, hasLength(2));
  });

  test(
    'structured native error preserves command, code, and message',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: {
          'audio_extract': {
            'ok': false,
            'error': {
              'code': 'NOT_FOUND',
              'message': 'sample not found: DIA_HERO_1',
            },
          },
        },
      );

      final error = await _captureModFfiException(
        ModFfi(core).audioExtract('speech.fsb', 'DIA_HERO_1'),
      );

      expect(error.command, 'audio_extract');
      expect(error.code, 'NOT_FOUND');
      expect(error.message, 'sample not found: DIA_HERO_1');
      expect(
        error.toString(),
        'audio_extract: sample not found: DIA_HERO_1 [NOT_FOUND]',
      );
    },
  );

  test(
    'malformed native error fields use one bounded local identity',
    () async {
      final oversizedCode = List.filled(129, 'A').join();
      final oversizedMessage = List.filled(64 * 1024 + 1, 'x').join();
      final multibyteCode = List.filled(65, 'Ã„').join();
      final multibyteMessage = List.filled(32 * 1024 + 1, 'Ã©').join();
      final malformedResponses = <Map<String, Object?>>[
        const {},
        const {'ok': 'false'},
        const {'ok': false},
        const {'ok': false, 'error': 'bad'},
        const {'ok': false, 'error': <String, Object?>{}},
        const {
          'ok': false,
          'error': {'code': 'IO', 'message': 'failure'},
          'extra': true,
        },
        const {
          'ok': false,
          'error': {'code': 'IO', 'message': 'failure', 'extra': true},
        },
        const {
          'ok': false,
          'error': {'code': 7, 'message': 'failure'},
        },
        const {
          'ok': false,
          'error': {'code': '', 'message': 'failure'},
        },
        const {
          'ok': false,
          'error': {'code': 'bad_code', 'message': 'failure'},
        },
        {
          'ok': false,
          'error': {'code': oversizedCode, 'message': 'failure'},
        },
        {
          'ok': false,
          'error': {'code': multibyteCode, 'message': 'failure'},
        },
        const {
          'ok': false,
          'error': {'code': 'IO'},
        },
        const {
          'ok': false,
          'error': {'code': 'IO', 'message': 7},
        },
        const {
          'ok': false,
          'error': {'code': 'IO', 'message': '  \n'},
        },
        {
          'ok': false,
          'error': {'code': 'IO', 'message': oversizedMessage},
        },
        {
          'ok': false,
          'error': {'code': 'IO', 'message': multibyteMessage},
        },
      ];

      for (final response in malformedResponses) {
        final core = FakeGoreCoreFfiService(responses: {'find_game': response});
        final error = await _captureModFfiException(ModFfi(core).findGameExe());

        expect(error.command, 'find_game');
        expect(error.code, ModFfiException.malformedNativeResponseCode);
        expect(error.message, startsWith('malformed native response:'));
        expect(error.message.length, lessThan(128));
        expect(error.toString(), isNot(contains(oversizedMessage)));
        expect(error.toString(), isNot(contains(oversizedCode)));
        expect(error.toString(), isNot(contains(multibyteMessage)));
        expect(error.toString(), isNot(contains(multibyteCode)));
      }
    },
  );

  test(
    'undecodable response gets the stable malformed response code',
    () async {
      final error = await _captureModFfiException(
        ModFfi(_MalformedJsonCoreService()).findGameExe(),
      );

      expect(error.command, 'find_game');
      expect(error.code, ModFfiException.malformedNativeResponseCode);
      expect(
        error.toString(),
        'find_game: malformed native response: response could not be decoded '
        '[MALFORMED_NATIVE_RESPONSE]',
      );
      expect(error.toString(), isNot(contains('hostile undecodable')));
    },
  );

  test(
    'scriptCompileReportV1 returns compiler failure as structured data',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: {
          'script_compile_report_v1': {
            'ok': true,
            'outcome': 'failed',
            'mini_path': null,
            'module': null,
            'compile_error': {
              'code': 'COMPILER_REGEN_FAILED',
              'message': 'compiler rejected the source',
            },
            'compiler_diagnostics': {
              'capture': 'captured',
              'messages': [
                {
                  'file': 'GoreMods/Probe.as',
                  'line': 5,
                  'column': 9,
                  'severity': 'error',
                  'message': 'Expected expression',
                },
              ],
              'omitted': 0,
            },
            'install_restore': 'restored_exact',
            'recovery_required': false,
          },
        },
      );

      final report = await ModFfi(core).scriptCompileReportV1(
        gameDir: r'C:\Game',
        op: 'add',
        moduleName: 'GoreMods.Probe',
        relPath: 'GoreMods/Probe.as',
        asPath: r'C:\Source\Probe.as',
        workDir: r'C:\Temp\compile',
        allowNewSymbols: true,
      );

      expect(report.compiled, isFalse);
      expect(report.failure!.code, 'COMPILER_REGEN_FAILED');
      expect(report.diagnostics!.messages.single.line, 5);
      expect(core.calls.single.command, 'script_compile_report_v1');
      expect(core.calls.single.payload['allow_new_symbols'], isTrue);
    },
  );

  test('scriptCompileReportV1 rejects a malformed success envelope', () async {
    final core = FakeGoreCoreFfiService(
      responses: {
        'script_compile_report_v1': {
          'ok': true,
          'outcome': 'compiled',
          'mini_path': 'mini.cache',
          'module': 'GoreMods.Probe',
          'compile_error': null,
          'compiler_diagnostics': null,
          'install_restore': 'restored_exact',
          'recovery_required': false,
        },
      },
    );

    final error = await _captureModFfiException(
      ModFfi(core).scriptCompileReportV1(
        gameDir: r'C:\Game',
        op: 'add',
        moduleName: 'GoreMods.Probe',
        relPath: 'GoreMods/Probe.as',
        asPath: r'C:\Source\Probe.as',
        workDir: r'C:\Temp\compile',
      ),
    );

    expect(error.code, ModFfiException.malformedNativeResponseCode);
    expect(
      error.message,
      'malformed native response: compile report schema is invalid',
    );
  });

  test(
    'scriptCompileReportV1 accepts only a marked direct owned output',
    () async {
      final work = Directory.systemTemp.createTempSync(
        'gore-owned-output-test-',
      );
      addTearDown(() => work.deleteSync(recursive: true));
      final owned = Directory(
        p.join(work.path, 'gore-owned-compile-a1b2c3d4e5f6'),
      )..createSync();
      File(
        p.join(owned.path, '.gore-owned-compile-v1'),
      ).writeAsStringSync('gore-owned-compile-staging-v1\n');
      final mini = File(p.join(owned.path, 'module.cache'))
        ..writeAsBytesSync(const [1, 2, 3]);
      final core = FakeGoreCoreFfiService(
        responses: {
          'script_compile_report_v1': _compiledScriptReport(mini.path),
        },
      );

      final report = await ModFfi(core).scriptCompileReportV1(
        gameDir: r'C:\Game',
        op: 'add',
        moduleName: 'GoreMods.Probe',
        relPath: 'GoreMods/Probe.as',
        asPath: r'C:\Source\Probe.as',
        workDir: work.path,
      );

      expect(report.compiled, isTrue);
      expect(report.miniPath, mini.path);
    },
  );

  test(
    'scriptCompileReportV1 maps missing ownership evidence to malformed',
    () async {
      final work = Directory.systemTemp.createTempSync(
        'gore-owned-output-test-',
      );
      addTearDown(() => work.deleteSync(recursive: true));
      final owned = Directory(
        p.join(work.path, 'gore-owned-compile-a1b2c3d4e5f6'),
      )..createSync();
      final mini = File(p.join(owned.path, 'module.cache'))
        ..writeAsBytesSync(const [1]);
      final core = FakeGoreCoreFfiService(
        responses: {
          'script_compile_report_v1': _compiledScriptReport(mini.path),
        },
      );

      final error = await _captureModFfiException(
        ModFfi(core).scriptCompileReportV1(
          gameDir: r'C:\Game',
          op: 'add',
          moduleName: 'GoreMods.Probe',
          relPath: 'GoreMods/Probe.as',
          asPath: r'C:\Source\Probe.as',
          workDir: work.path,
        ),
      );

      expect(error.code, ModFfiException.malformedNativeResponseCode);
    },
  );

  test(
    'scriptCompileReportV1 rejects an extended-prefix output response',
    () async {
      final work = Directory.systemTemp.createTempSync(
        'gore-owned-output-test-',
      );
      addTearDown(() => work.deleteSync(recursive: true));
      final owned = Directory(
        p.join(work.path, 'gore-owned-compile-a1b2c3d4e5f6'),
      )..createSync();
      File(
        p.join(owned.path, '.gore-owned-compile-v1'),
      ).writeAsStringSync('gore-owned-compile-staging-v1\n');
      final mini = File(p.join(owned.path, 'module.cache'))
        ..writeAsBytesSync(const [1]);
      final core = FakeGoreCoreFfiService(
        responses: {
          'script_compile_report_v1': _compiledScriptReport(
            '\\\\?\\${mini.path}',
          ),
        },
      );

      final error = await _captureModFfiException(
        ModFfi(core).scriptCompileReportV1(
          gameDir: r'C:\Game',
          op: 'add',
          moduleName: 'GoreMods.Probe',
          relPath: 'GoreMods/Probe.as',
          asPath: r'C:\Source\Probe.as',
          workDir: work.path,
        ),
      );

      expect(error.code, ModFfiException.malformedNativeResponseCode);
    },
  );

  test('scriptCompileReportV1 rejects non-native owned-child shapes', () async {
    final work = Directory.systemTemp.createTempSync('gore-owned-output-test-');
    addTearDown(() => work.deleteSync(recursive: true));
    final candidates = <Directory>[
      Directory(p.join(work.path, 'nested', 'gore-owned-compile-a1b2c3d4e5f6')),
      Directory(p.join(work.path, 'gore-owned-compile-A1b2c3d4e5f6')),
    ];
    for (final candidate in candidates) {
      candidate.createSync(recursive: true);
      File(
        p.join(candidate.path, '.gore-owned-compile-v1'),
      ).writeAsStringSync('gore-owned-compile-staging-v1\n');
      final mini = File(p.join(candidate.path, 'module.cache'))
        ..writeAsBytesSync(const [1]);
      final core = FakeGoreCoreFfiService(
        responses: {
          'script_compile_report_v1': _compiledScriptReport(mini.path),
        },
      );

      final error = await _captureModFfiException(
        ModFfi(core).scriptCompileReportV1(
          gameDir: r'C:\Game',
          op: 'add',
          moduleName: 'GoreMods.Probe',
          relPath: 'GoreMods/Probe.as',
          asPath: r'C:\Source\Probe.as',
          workDir: work.path,
        ),
      );

      expect(
        error.code,
        ModFfiException.malformedNativeResponseCode,
        reason: candidate.path,
      );
    }
  });

  test(
    'scriptCompileInstallStateV1 sends the root and parses safety',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: {
          'script_compile_install_state_v1': <String, Object?>{
            'ok': true,
            'disposition': 'safe_to_compile',
            'safe_to_compile': true,
            'game_process': 'not_running',
            'artifacts': <Object?>[],
            'issues': <Object?>[],
          },
        },
      );

      final state = await ModFfi(
        core,
      ).scriptCompileInstallStateV1(gameDir: r'C:\Game');

      expect(state.safeToCompile, isTrue);
      expect(core.calls.single.command, 'script_compile_install_state_v1');
      expect(core.calls.single.payload, <String, Object?>{
        'game_dir': r'C:\Game',
      });
    },
  );

  test(
    'voiceArchiveMatchLine sends the command and parses a strict result',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: {'voice_archive_match_line': _validVoiceMatchResponse()},
      );

      final result = await ModFfi(core).voiceArchiveMatchLine(
        archive: r'C:\Game\german.zip',
        locId: 'LINE_ONE',
      );

      expect(result.resolution, VoiceArchiveLineResolution.unique);
      expect(result.archiveSize, 4096);
      expect(result.matches.single.path, 'Voices/Hero/line_one.OGG');
      expect(core.calls.single.command, 'voice_archive_match_line');
      expect(core.calls.single.payload, {
        'archive': r'C:\Game\german.zip',
        'loc_id': 'LINE_ONE',
      });
    },
  );

  test(
    'voiceOggInspectV1 sends only the selected path and parses facts',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: {'voice_ogg_inspect_v1': _validVoiceOggInspectResponse()},
      );

      final result = await ModFfi(
        core,
      ).voiceOggInspectV1(oggPath: r'C:\Recordings\viper.ogg');

      expect(result.codec, VoiceOggCodec.vorbis);
      expect(result.pages, 2);
      expect(result.streams, 1);
      expect(result.contentSeal.byteLength, 4096);
      expect(result.contentSeal.sha256, List.filled(64, 'a').join());
      expect(core.calls.single.command, 'voice_ogg_inspect_v1');
      expect(core.calls.single.payload, {
        'ogg_path': r'C:\Recordings\viper.ogg',
      });
    },
  );

  test('voice Ogg request bounds fail locally before core execution', () async {
    final core = FakeGoreCoreFfiService(
      responses: {'voice_ogg_inspect_v1': _validVoiceOggInspectResponse()},
    );
    final ffi = ModFfi(core);

    for (final path in <String>[
      '',
      'bad\u0000path.ogg',
      List.filled(32 * 1024 + 1, 'x').join(),
      List.filled(10923, '\u20ac').join(),
      String.fromCharCode(0xd800),
    ]) {
      await expectLater(
        ffi.voiceOggInspectV1(oggPath: path),
        throwsArgumentError,
      );
    }
    expect(core.calls, isEmpty);
  });

  test(
    'voice Ogg request accepts the native escaped-envelope boundary',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: {'voice_ogg_inspect_v1': _validVoiceOggInspectResponse()},
      );
      final boundaryPath = List.filled(32 * 1024, '\u0001').join();

      await ModFfi(core).voiceOggInspectV1(oggPath: boundaryPath);

      expect(core.calls.single.payload, {'ogg_path': boundaryPath});
    },
  );

  test('voice Ogg inspection DTO rejects non-exact or implausible facts', () {
    final malformed = <void Function(Map<String, Object?>)>[
      (response) => response['extra'] = true,
      (response) => response.remove('streams'),
      (response) => response['ok'] = false,
      (response) => response['codec'] = 'mp3',
      (response) => response['pages'] = 0,
      (response) => response['pages'] = 1.5,
      (response) => response['pages'] = 0x100000000,
      (response) => response['streams'] = 0,
      (response) => response['streams'] = 3,
      (response) => response['content_seal'] = 'not-an-object',
      (response) =>
          (response['content_seal'] as Map<String, Object?>)['extra'] = true,
      (response) =>
          (response['content_seal'] as Map<String, Object?>)['byte_len'] = 26,
      (response) =>
          (response['content_seal'] as Map<String, Object?>)['byte_len'] =
              64 * 1024 * 1024 + 1,
      (response) =>
          (response['content_seal'] as Map<String, Object?>)['sha256'] =
              List.filled(64, 'A').join(),
      (response) {
        response['pages'] = 2;
        (response['content_seal'] as Map<String, Object?>)['byte_len'] = 53;
      },
    ];

    for (final mutate in malformed) {
      final response = _validVoiceOggInspectResponse();
      mutate(response);
      expect(
        () => VoiceOggInspectionResult.fromJson(response),
        throwsFormatException,
      );
    }
  });

  test('voice Ogg inspection DTO accepts the closed Opus codec', () {
    final response = _validVoiceOggInspectResponse()..['codec'] = 'opus';

    expect(
      VoiceOggInspectionResult.fromJson(response).codec,
      VoiceOggCodec.opus,
    );
  });

  test(
    'Story catalog wrapper preserves raw input and parses immutable choices',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: {
          'authoring_story_catalog_v1_read': _validStoryCatalogResponse(),
        },
      );

      final result = await ModFfi(
        core,
      ).authoringStoryCatalogV1Read(catalogJson: _storyCatalogRequest);

      expect(core.calls, hasLength(1));
      expect(core.calls.single.command, 'authoring_story_catalog_v1_read');
      expect(core.calls.single.payload, {'catalog_json': _storyCatalogRequest});
      expect(result.schemaRevision, 1);
      expect(result.generation.edition, 'g1r-steam');
      expect(result.npcs.map((entry) => entry.displayName), [
        'Asghan',
        'Viper',
      ]);
      expect(result.npcs.first.runtimeUniqueName, 'OM_GRD_Asghan_263');
      expect(
        result.npcs.first.authoringQualification,
        AuthoringStoryCatalogNpcAuthoringQualification.offlineQualified,
      );
      expect(
        result.npcs.first.runtimeQualification,
        AuthoringStoryCatalogRuntimeQualification.runtimeUnqualified,
      );
      expect(
        result.questParents.single.questClass.runtimeClass,
        'UQuest_SwampCamp_SCCHAPTER2',
      );
      expect(
        result.questCollisionCatalog.status,
        AuthoringStoryCatalogCollisionStatus.inventoryUnavailable,
      );
      expect(result.questCollisionCatalog.blocksDraftCreation, isTrue);
      expect(result.blocksBuild, isTrue);
      expect(() => result.npcs.clear(), throwsUnsupportedError);
      expect(() => result.questParents.clear(), throwsUnsupportedError);
    },
  );

  test(
    'Story catalog build binds exact paths and can feed the pinned reader',
    () async {
      expect(
        _storyCatalogBuildBinding(
          executable: 'A/game.exe',
          shippingCache: 'B/Shipping-G1-Game.cache',
          bindsCache: 'C/Binds.cache',
        ),
        '86c32f29c17846499a62e6acf9778610fe25b445930519e6e055aa427519cb37',
      );
      final rawCatalog = _storyCatalogBuildRaw();
      final core = FakeGoreCoreFfiService(
        responses: {
          'authoring_story_catalog_v1_build': _validStoryCatalogBuildResponse(
            catalogJson: rawCatalog,
          ),
          'authoring_story_catalog_v1_read': _validStoryCatalogResponse(
            catalogJson: rawCatalog,
          ),
        },
      );
      final ffi = ModFfi(core);

      final built = await ffi.authoringStoryCatalogV1Build(
        executable: _storyCatalogExecutable,
        shippingCache: _storyCatalogShippingCache,
        bindsCache: _storyCatalogBindsCache,
      );
      final selections = await ffi.authoringStoryCatalogV1BuildAndRead(
        executable: _storyCatalogExecutable,
        shippingCache: _storyCatalogShippingCache,
        bindsCache: _storyCatalogBindsCache,
      );

      expect(built.catalogJson, rawCatalog);
      expect(built.generation.edition, 'g1r-steam');
      expect(built.catalogSeal.sha256, List.filled(64, '4').join());
      expect(selections.npcs, hasLength(2));
      expect(core.calls.map((call) => call.command), <String>[
        'authoring_story_catalog_v1_build',
        'authoring_story_catalog_v1_build',
        'authoring_story_catalog_v1_read',
      ]);
      expect(core.calls.first.payload, <String, Object?>{
        'executable': _storyCatalogExecutable,
        'shipping_cache': _storyCatalogShippingCache,
        'binds_cache': _storyCatalogBindsCache,
      });
      expect(core.calls.last.payload, <String, Object?>{
        'catalog_json': rawCatalog,
      });
    },
  );

  test(
    'Story catalog build rejects response confusion and loose JSON',
    () async {
      final malformed = <void Function(Map<String, Object?>)>[
        (response) => response['extra'] = true,
        (response) =>
            response['request_binding_sha256'] = List.filled(64, 'a').join(),
        (response) => response['request_binding_sha256'] =
            _storyCatalogBuildBinding().toUpperCase(),
        (response) => response['catalog_json'] = '${_storyCatalogBuildRaw()}\n',
        (response) =>
            response['catalog_json'] = _storyCatalogBuildRaw().replaceFirst(
              '"format":"story_catalog"',
              '"format":"story_catalog","format":"story_catalog"',
            ),
        (response) =>
            (response['generation'] as Map<String, Object?>)['edition'] =
                'g1r-other',
        (response) =>
            ((response['generation'] as Map<String, Object?>)['executable']
                    as Map<String, Object?>)['byte_len'] =
                1,
        (response) =>
            (response['catalog_seal'] as Map<String, Object?>)['sha256'] =
                List.filled(64, 'f').join(),
        (response) {
          final raw =
              jsonDecode(response['catalog_json'] as String)
                  as Map<String, Object?>;
          (raw['catalog'] as Map<String, Object?>)['extra'] = true;
          response['catalog_json'] = jsonEncode(raw);
        },
        (response) => response['catalog_json'] = '[]',
      ];

      for (final mutate in malformed) {
        final response = _validStoryCatalogBuildResponse();
        mutate(response);
        final core = FakeGoreCoreFfiService(
          responses: {'authoring_story_catalog_v1_build': response},
        );
        await expectLater(
          ModFfi(core).authoringStoryCatalogV1Build(
            executable: _storyCatalogExecutable,
            shippingCache: _storyCatalogShippingCache,
            bindsCache: _storyCatalogBindsCache,
          ),
          throwsFormatException,
        );
      }
    },
  );

  test('Story catalog build bounds paths before FFI', () async {
    final core = FakeGoreCoreFfiService(
      responses: {
        'authoring_story_catalog_v1_build': _validStoryCatalogBuildResponse(),
      },
    );
    final ffi = ModFfi(core);
    for (final executable in <String>[
      '',
      'bad\u0000path',
      List.filled(32 * 1024 + 1, 'x').join(),
      String.fromCharCode(0xd800),
    ]) {
      await expectLater(
        ffi.authoringStoryCatalogV1Build(
          executable: executable,
          shippingCache: _storyCatalogShippingCache,
          bindsCache: _storyCatalogBindsCache,
        ),
        throwsArgumentError,
      );
    }
    expect(core.calls, isEmpty);
  });

  test(
    'Story catalog game-root build is root-bound and feeds the strict reader',
    () async {
      expect(
        _storyCatalogGameRootBinding(),
        '208d76c5754bc4457ea54b30605d1081b21894d3d8ea925c5e925257da370f7b',
      );
      final rawCatalog = _storyCatalogBuildRaw();
      final core = FakeGoreCoreFfiService(
        responses: {
          'authoring_story_catalog_v1_build_for_game_root':
              _validStoryCatalogGameRootBuildResponse(catalogJson: rawCatalog),
          'authoring_story_catalog_v1_read': _validStoryCatalogResponse(
            catalogJson: rawCatalog,
          ),
        },
      );
      final result = await ModFfi(core)
          .authoringStoryCatalogV1BuildAndReadForGameRoot(
            gameRoot: _storyCatalogGameRoot,
          );

      expect(result.npcs, hasLength(2));
      expect(core.calls.map((call) => call.command), <String>[
        'authoring_story_catalog_v1_build_for_game_root',
        'authoring_story_catalog_v1_read',
      ]);
      expect(core.calls.first.payload, <String, Object?>{
        'game_root': _storyCatalogGameRoot,
      });
      expect(core.calls.last.payload, <String, Object?>{
        'catalog_json': rawCatalog,
      });
    },
  );

  test(
    'Story catalog game-root build rejects wrong binding and bad roots',
    () async {
      final confused = _validStoryCatalogGameRootBuildResponse()
        ..['request_binding_sha256'] = _storyCatalogBuildBinding();
      await expectLater(
        ModFfi(
          FakeGoreCoreFfiService(
            responses: {
              'authoring_story_catalog_v1_build_for_game_root': confused,
            },
          ),
        ).authoringStoryCatalogV1BuildForGameRoot(
          gameRoot: _storyCatalogGameRoot,
        ),
        throwsFormatException,
      );

      final core = FakeGoreCoreFfiService(
        responses: {
          'authoring_story_catalog_v1_build_for_game_root':
              _validStoryCatalogGameRootBuildResponse(),
        },
      );
      for (final root in <String>[
        '',
        'bad\u0000root',
        List.filled(32 * 1024 + 1, 'x').join(),
        String.fromCharCode(0xd800),
      ]) {
        await expectLater(
          ModFfi(core).authoringStoryCatalogV1BuildForGameRoot(gameRoot: root),
          throwsArgumentError,
        );
      }
      expect(core.calls, isEmpty);
    },
  );

  test(
    'Story catalog DTO rejects unbound, loose, and inconsistent data',
    () async {
      final malformed = <void Function(Map<String, Object?>)>[
        (response) => response['extra'] = true,
        (response) =>
            response['request_catalog_sha256'] = List.filled(64, 'a').join(),
        (response) => response['request_catalog_sha256'] = crypto.sha256
            .convert(utf8.encode(_storyCatalogRequest))
            .toString()
            .toUpperCase(),
        (response) =>
            (response['selections'] as Map<String, Object?>)['extra'] = true,
        (response) =>
            (response['selections']
                    as Map<String, Object?>)['schema_revision'] =
                2,
        (response) {
          final npcs =
              (response['selections'] as Map<String, Object?>)['npcs'] as List;
          final reversed = npcs.reversed.toList();
          npcs.setAll(0, reversed);
        },
        (response) {
          final npcs =
              (response['selections'] as Map<String, Object?>)['npcs'] as List;
          final first = npcs[0] as Map<String, Object?>;
          final second = npcs[1] as Map<String, Object?>;
          (second['character_definition']
                  as Map<String, Object?>)['authoring_selector'] =
              (first['character_definition']
                  as Map<String, Object?>)['authoring_selector'];
        },
        (response) {
          final npcs =
              (response['selections'] as Map<String, Object?>)['npcs'] as List;
          final first = npcs[0] as Map<String, Object?>;
          (first['character_definition']
                  as Map<String, Object?>)['authoring_selector'] =
              'Catalog_${List.filled(64, '0').join()}';
        },
        (response) {
          final npcs =
              (response['selections'] as Map<String, Object?>)['npcs'] as List;
          final first = npcs[0] as Map<String, Object?>;
          final character =
              first['character_definition'] as Map<String, Object?>;
          final ai = first['ai_agent_config'] as Map<String, Object?>;
          final alias = character['authoring_selector'];
          character['authoring_selector'] = ai['authoring_selector'];
          ai['authoring_selector'] = alias;
        },
        (response) {
          final npcs =
              (response['selections'] as Map<String, Object?>)['npcs'] as List;
          final first = npcs[0] as Map<String, Object?>;
          (first['character_definition']
                  as Map<String, Object?>)['source_catalog_selector'] =
              'script-class:Trusted/UCharacterDefinition_Human_OTHER';
        },
        (response) {
          final npcs =
              (response['selections'] as Map<String, Object?>)['npcs'] as List;
          final first = npcs[0] as Map<String, Object?>;
          (first['character_definition']
                  as Map<String, Object?>)['catalog_layer'] =
              'base-game.g1r.other';
        },
        (response) {
          final npcs =
              (response['selections'] as Map<String, Object?>)['npcs'] as List;
          (npcs[0] as Map<String, Object?>)['runtime_qualification'] =
              'runtime_qualified';
        },
        (response) {
          final npcs =
              (response['selections'] as Map<String, Object?>)['npcs'] as List;
          (npcs[0] as Map<String, Object?>)['display_name'] = ' Asghan';
        },
        (response) {
          final selections = response['selections'] as Map<String, Object?>;
          final collision =
              selections['quest_collision_catalog'] as Map<String, Object?>;
          collision['source_seal'] = _catalogContentSeal('f', 123394250);
        },
        (response) {
          final selections = response['selections'] as Map<String, Object?>;
          final collision =
              selections['quest_collision_catalog'] as Map<String, Object?>;
          collision['blocks_draft_creation'] = false;
        },
        (response) =>
            (response['selections'] as Map<String, Object?>)['blocks_build'] =
                false,
        (response) {
          final selections = response['selections'] as Map<String, Object?>;
          final parents = selections['quest_parents'] as List;
          final questClass =
              (parents.single as Map<String, Object?>)['quest_class']
                  as Map<String, Object?>;
          questClass['source_catalog_selector'] = r'script-class:Bad\Path';
        },
        (response) {
          final selections = response['selections'] as Map<String, Object?>;
          final generation = selections['generation'] as Map<String, Object?>;
          (generation['executable'] as Map<String, Object?>)['byte_len'] = 1.0;
        },
      ];

      for (final mutate in malformed) {
        final response = _validStoryCatalogResponse();
        mutate(response);
        final core = FakeGoreCoreFfiService(
          responses: {'authoring_story_catalog_v1_read': response},
        );
        await expectLater(
          ModFfi(
            core,
          ).authoringStoryCatalogV1Read(catalogJson: _storyCatalogRequest),
          throwsFormatException,
        );
      }
    },
  );

  test(
    'Story catalog wrapper bounds raw and escaped inputs before FFI',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: {
          'authoring_story_catalog_v1_read': _validStoryCatalogResponse(),
        },
      );
      final ffi = ModFfi(core);

      await expectLater(
        ffi.authoringStoryCatalogV1Read(
          catalogJson: String.fromCharCodes(Uint8List(16 * 1024 * 1024 + 1)),
        ),
        throwsArgumentError,
      );
      await expectLater(
        ffi.authoringStoryCatalogV1Read(
          catalogJson: String.fromCharCodes(Uint8List(11 * 1024 * 1024)),
        ),
        throwsArgumentError,
      );
      expect(core.calls, isEmpty);
    },
  );

  test(
    'voice match DTO rejects fractional, negative, and out-of-range integers',
    () {
      final malformed = <void Function(Map<String, Object?>)>[
        (response) => response['match_count'] = 1.5,
        (response) => response['archive_size'] = -1,
        (response) => _firstVoiceMatch(response)['index'] = -1,
        (response) => _firstVoiceMatch(response)['compressed_size'] = 1.5,
        (response) => _firstVoiceMatch(response)['crc32'] = 0x100000000,
        (response) => _firstVoiceMatch(response)['compression_code'] = 0x10000,
        (response) => _firstVoiceMatch(response)['unix_mode'] = -1,
        (response) => _voiceTimestamp(response)['month'] = 13,
        (response) {
          _voiceTimestamp(response)['month'] = 2;
          _voiceTimestamp(response)['day'] = 31;
        },
      ];

      for (final mutate in malformed) {
        final response = _validVoiceMatchResponse();
        mutate(response);
        expect(
          () => VoiceArchiveMatchLineResult.fromJson(response),
          throwsFormatException,
        );
      }
    },
  );

  test('voice match DTO rejects inconsistent or ineligible match metadata', () {
    final malformed = <void Function(Map<String, Object?>)>[
      (response) => response['expected_basename'] = 'OTHER.ogg',
      (response) => response['loc_id'] = 'LÃNE_ONE',
      (response) => _firstVoiceMatch(response)['basename'] = 'OTHER.ogg',
      (response) =>
          _firstVoiceMatch(response)['path'] = 'Voices/Hero/OTHER.ogg',
      (response) =>
          _firstVoiceMatch(response)['path'] = r'Voices\Hero\line_one.OGG',
      (response) => _firstVoiceMatch(response)['is_symlink'] = true,
      (response) => _firstVoiceMatch(response)['encrypted'] = true,
      (response) => _firstVoiceMatch(response)['compression_code'] = 12,
      (response) => _firstVoiceMatch(response)['compression'] = 'deflated',
    ];

    for (final mutate in malformed) {
      final response = _validVoiceMatchResponse();
      mutate(response);
      expect(
        () => VoiceArchiveMatchLineResult.fromJson(response),
        throwsFormatException,
      );
    }
  });

  test('working-head DTO accepts only exact canonical bounded bytes', () {
    final valid = AuthoringWorkingHead.fromCanonicalJson(
      _validWorkingHeadJson(),
    );
    expect(valid.snapshotByteLength, 321);
    expect(valid.snapshotSha256, List.filled(64, 'a').join());
    final maximumRevision3 = AuthoringWorkingHead.fromCanonicalJson(
      _validWorkingHeadJson().replaceFirst(
        '"byte_len":321',
        '"byte_len":${17 * 1024 * 1024}',
      ),
    );
    expect(maximumRevision3.snapshotByteLength, 17 * 1024 * 1024);

    final malformed = <String>[
      '{}',
      ' ${_validWorkingHeadJson()}',
      _validWorkingHeadJson().replaceFirst(
        '"store_format":1',
        '"store_format":2',
      ),
      _validWorkingHeadJson().replaceFirst(
        '"store_format":1',
        '"store_format":1.0',
      ),
      _validWorkingHeadJson().replaceFirst('"byte_len":321', '"byte_len":0'),
      _validWorkingHeadJson().replaceFirst(
        '"byte_len":321',
        '"byte_len":${17 * 1024 * 1024 + 1}',
      ),
      _validWorkingHeadJson().replaceFirst(
        List.filled(64, 'a').join(),
        List.filled(64, 'A').join(),
      ),
      _validWorkingHeadJson().replaceFirst(
        '"store_format":1',
        '"store_format":1,"store_format":1',
      ),
      List.filled(64 * 1024 + 1, 'x').join(),
    ];
    for (final value in malformed) {
      expect(
        () => AuthoringWorkingHead.fromCanonicalJson(value),
        throwsFormatException,
      );
    }
  });
}
