import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

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

Map<String, Object?> _firstVoiceMatch(Map<String, Object?> response) =>
    ((response['matches'] as List<Object?>).single as Map)
        .cast<String, Object?>();

Map<String, Object?> _voiceTimestamp(Map<String, Object?> response) =>
    (_firstVoiceMatch(response)['last_modified'] as Map)
        .cast<String, Object?>();

Map<String, Object?> _validAuthoringCheckResponse() => {
  'ok': true,
  'canonical_project_json':
      '{"format":2,"schema_revision":1,"project_id":"00000000000000000000000000000001"}',
  'diagnostics': <Object?>[
    <String, Object?>{
      'code': 'INVALID_GENERATION_ANCHOR',
      'severity': 'error',
      'entity': null,
      'property_path': 'target.executable.byte_len',
      'message':
          'game generation executable seal must have a non-zero byte length',
      'related_entities': <Object?>[],
      'blocks_build': true,
    },
    <String, Object?>{
      'code': 'UNQUALIFIED_VOICE_ADD',
      'severity': 'warning',
      'entity': '00000000000000000000000000000001',
      'property_path': 'payload.data.target_resolution.target.operation',
      'message': 'new voice-member runtime binding is not qualified',
      'related_entities': <Object?>[
        '00000000000000000000000000000002',
        '00000000000000000000000000000003',
      ],
      'blocks_build': false,
    },
  ],
  'blocks_build': true,
};

String _validWorkingHeadJson() =>
    '{"store_format":1,"snapshot":{"byte_len":321,"sha256":"${List.filled(64, 'a').join()}"}}';

String _validCanonicalProjectJson() =>
    '{"format":2,"schema_revision":1,'
    '"project_id":"00000000000000000000000000000001","revision":0,'
    '"meta":{"name":"Store bridge","version":"1.0.0","author":"tests"},'
    '"target":{"executable":{"byte_len":1,'
    '"sha256":"${List.filled(64, '4').join()}"}},'
    '"authoring_locales":[],"entities":{},"asset_store":{"assets":{}}}';

Map<String, Object?> _validStoreOpenedResponse() => {
  'ok': true,
  'head_json': _validWorkingHeadJson(),
  'project_json': _validCanonicalProjectJson(),
  'diagnostics': <Object?>[],
  'blocks_build': false,
};

Map<String, Object?> _validCheckpointPreparationResponse() => {
  'ok': true,
  'head_json': _validWorkingHeadJson(),
  'diagnostics': <Object?>[],
  'blocks_build': false,
};

Map<String, Object?> _validImportedOggResponse() => {
  'ok': true,
  'asset': <String, Object?>{
    'sha256': List.filled(64, 'b').join(),
    'byte_len': 4096,
    'logical_name': 'voice/asghan.ogg',
  },
  'ogg': <String, Object?>{
    'codec': 'vorbis',
    'channels': 1,
    'sample_rate': 48000,
    'pages': 3,
    'logical_streams': 1,
  },
  'deduplicated': false,
};

Map<String, Object?> _authoringDiagnostic(
  Map<String, Object?> response,
  int index,
) => ((response['diagnostics'] as List<Object?>)[index] as Map)
    .cast<String, Object?>();

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
      final multibyteCode = List.filled(65, 'Ä').join();
      final multibyteMessage = List.filled(32 * 1024 + 1, 'é').join();
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

  test('scriptCompile propagates the new-symbol opt-in', () async {
    final core = FakeGoreCoreFfiService(
      responses: {
        'script_compile': {
          'ok': true,
          'mini_path': 'mini.cache',
          'module': 'GoreMods.Probe',
        },
      },
    );

    await ModFfi(core).scriptCompile(
      gameDir: r'C:\Game',
      op: 'add',
      moduleName: 'GoreMods.Probe',
      relPath: 'GoreMods/Probe.as',
      asPath: r'C:\Source\Probe.as',
      workDir: r'C:\Temp\compile',
      allowNewSymbols: true,
    );

    expect(core.calls, hasLength(1));
    expect(core.calls.single.command, 'script_compile');
    expect(core.calls.single.payload['allow_new_symbols'], isTrue);
  });

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
    'authoringProjectCheck preserves raw JSON and uses a closed profile',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: {'authoring_project_check': _validAuthoringCheckResponse()},
      );
      const rawProject = '{"revision":0,"revision":1}';

      final result = await ModFfi(core).authoringProjectCheck(
        projectJson: rawProject,
        profile: AuthoringValidationProfile.experimental,
      );

      expect(core.calls, hasLength(1));
      expect(core.calls.single.command, 'authoring_project_check');
      expect(core.calls.single.payload, {
        'project_json': rawProject,
        'profile': 'experimental',
      });
      expect(result.blocksBuild, isTrue);
      expect(result.diagnostics, hasLength(2));
      expect(
        result.diagnostics.first.severity,
        AuthoringDiagnosticSeverity.error,
      );
      expect(
        result.diagnostics.last.entity,
        '00000000000000000000000000000001',
      );
      expect(
        () => result.diagnostics.clear(),
        throwsA(isA<UnsupportedError>()),
      );
      expect(
        () => result.diagnostics.last.relatedEntities.clear(),
        throwsA(isA<UnsupportedError>()),
      );
    },
  );

  test('authoring DTO rejects malformed and inconsistent wire data', () {
    final malformed = <void Function(Map<String, Object?>)>[
      (response) => response['canonical_project_json'] = '',
      (response) => response['diagnostics'] = <String, Object?>{},
      (response) => (response['diagnostics'] as List<Object?>)[0] = 'bad',
      (response) => _authoringDiagnostic(response, 0)['code'] = 'bad_code',
      (response) => _authoringDiagnostic(response, 0)['severity'] = 'fatal',
      (response) => _authoringDiagnostic(response, 1)['entity'] =
          '0000000000000000000000000000000A',
      (response) => _authoringDiagnostic(response, 0).remove('entity'),
      (response) => _authoringDiagnostic(response, 0)['property_path'] = '',
      (response) => _authoringDiagnostic(response, 0)['message'] = '',
      (response) =>
          _authoringDiagnostic(response, 1)['related_entities'] = <Object?>[
            '00000000000000000000000000000003',
            '00000000000000000000000000000002',
          ],
      (response) => _authoringDiagnostic(response, 0)['blocks_build'] = 'true',
      (response) => response['blocks_build'] = false,
      (response) => response['blocks_build'] = 1,
    ];

    for (final mutate in malformed) {
      final response = _validAuthoringCheckResponse();
      mutate(response);
      expect(
        () => AuthoringProjectCheckResult.fromJson(response),
        throwsFormatException,
      );
    }
  });

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
      (response) => response['loc_id'] = 'LÍNE_ONE',
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

  test(
    'working-store wrappers preserve raw CAS/project bytes and typed payloads',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: {
          'authoring_store_open': _validStoreOpenedResponse(),
          'authoring_store_prepare_checkpoint':
              _validCheckpointPreparationResponse(),
          'authoring_store_open_head_bytes': _validStoreOpenedResponse(),
          'authoring_store_import_ogg': _validImportedOggResponse(),
          'authoring_store_verify_asset': {'ok': true},
        },
      );
      final ffi = ModFfi(core);
      final head = AuthoringWorkingHead.fromCanonicalJson(
        _validWorkingHeadJson(),
      );
      const rawProject = '{"revision":0,"revision":1}';

      final opened = await ffi.authoringStoreOpen(
        root: r'C:\Mods\MyMod.goreproj',
        verification: AuthoringAssetVerification.full,
        profile: AuthoringValidationProfile.production,
      );
      final prepared = await ffi.authoringStorePrepareCheckpoint(
        root: r'C:\Mods\MyMod.goreproj',
        expectedHead: null,
        projectJson: rawProject,
        profile: AuthoringValidationProfile.experimental,
      );
      final candidate = await ffi.authoringStoreOpenHeadBytes(
        root: r'C:\Mods\MyMod.goreproj',
        head: head,
        verification: AuthoringAssetVerification.structural,
        profile: AuthoringValidationProfile.experimental,
      );
      final imported = await ffi.authoringStoreImportOgg(
        root: r'C:\Mods\MyMod.goreproj',
        source: r'C:\Recordings\asghan.ogg',
        logicalName: 'voice/asghan.ogg',
        expectedHead: head,
      );
      await ffi.authoringStoreVerifyAsset(
        root: r'C:\Mods\MyMod.goreproj',
        asset: imported.asset,
        verification: AuthoringAssetVerification.full,
      );

      expect(opened.head.canonicalJson, _validWorkingHeadJson());
      expect(prepared.head.snapshotByteLength, 321);
      expect(candidate.projectJson, _validCanonicalProjectJson());
      expect(imported.ogg.codec, AuthoringOggCodec.vorbis);
      expect(imported.asset.logicalName, 'voice/asghan.ogg');
      expect(core.calls, hasLength(5));
      expect(core.calls[1].command, 'authoring_store_prepare_checkpoint');
      expect(core.calls[1].payload['project_json'], rawProject);
      expect(core.calls[1].payload['expected_head_json'], isNull);
      expect(core.calls[2].payload['head_json'], _validWorkingHeadJson());
      expect(
        core.calls[3].payload['expected_head_json'],
        _validWorkingHeadJson(),
      );
      expect(core.calls[4].payload['asset'], imported.asset.toJson());
    },
  );

  test('working-head DTO accepts only exact canonical bounded bytes', () {
    final valid = AuthoringWorkingHead.fromCanonicalJson(
      _validWorkingHeadJson(),
    );
    expect(valid.snapshotByteLength, 321);
    expect(valid.snapshotSha256, List.filled(64, 'a').join());

    final malformed = <String>[
      '{}',
      ' ${_validWorkingHeadJson()}',
      _validWorkingHeadJson().replaceFirst(
        '"store_format":1',
        '"store_format":2',
      ),
      _validWorkingHeadJson().replaceFirst('"byte_len":321', '"byte_len":0'),
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

  test('working-store response DTOs reject loose or inconsistent data', () {
    final badOpen = <void Function(Map<String, Object?>)>[
      (response) => response['extra'] = true,
      (response) => response['head_json'] = ' ${_validWorkingHeadJson()}',
      (response) => response['project_json'] = '[]',
      (response) =>
          response['project_json'] = ' ${_validCanonicalProjectJson()}',
      (response) => response['project_json'] = _validCanonicalProjectJson()
          .replaceFirst('"revision":0', '"revision":0,"revision":0'),
      (response) =>
          response['project_json'] = _validCanonicalProjectJson().replaceFirst(
            '"format":2,"schema_revision":1',
            '"schema_revision":1,"format":2',
          ),
      (response) => response['diagnostics'] = <Object?>[
        _validAuthoringCheckResponse()['diagnostics'] as List<Object?>,
      ],
      (response) => response['blocks_build'] = true,
    ];
    for (final mutate in badOpen) {
      final response = _validStoreOpenedResponse();
      mutate(response);
      expect(
        () => AuthoringStoreOpenedResult.fromJson(response),
        throwsFormatException,
      );
    }

    final preparation = _validCheckpointPreparationResponse()
      ..['unexpected'] = true;
    expect(
      () => AuthoringCheckpointPreparation.fromJson(preparation),
      throwsFormatException,
    );

    final badImports = <void Function(Map<String, Object?>)>[
      (response) => response['extra'] = true,
      (response) => (response['asset'] as Map<String, Object?>)['byte_len'] = 0,
      (response) => (response['ogg'] as Map<String, Object?>)['codec'] = 'mp3',
      (response) => response['deduplicated'] = 0,
    ];
    for (final mutate in badImports) {
      final response = _validImportedOggResponse();
      mutate(response);
      expect(
        () => AuthoringImportedOgg.fromJson(response),
        throwsFormatException,
      );
    }
  });

  test('working-store request bounds reject locally before FFI', () async {
    final core = FakeGoreCoreFfiService(
      responses: {
        'authoring_store_open': _validStoreOpenedResponse(),
        'authoring_store_prepare_checkpoint':
            _validCheckpointPreparationResponse(),
        'authoring_store_import_ogg': _validImportedOggResponse(),
      },
    );
    final ffi = ModFfi(core);

    await expectLater(
      ffi.authoringStoreOpen(
        root: List.filled(32 * 1024 + 1, 'x').join(),
        verification: AuthoringAssetVerification.full,
        profile: AuthoringValidationProfile.production,
      ),
      throwsArgumentError,
    );
    await expectLater(
      ffi.authoringStorePrepareCheckpoint(
        root: 'root',
        expectedHead: null,
        projectJson: List.filled(16 * 1024 * 1024 + 1, 'x').join(),
        profile: AuthoringValidationProfile.production,
      ),
      throwsArgumentError,
    );
    await expectLater(
      ffi.authoringStoreImportOgg(
        root: 'root',
        source: 'voice.ogg',
        logicalName: List.filled(1025, 'x').join(),
        expectedHead: null,
      ),
      throwsArgumentError,
    );
    expect(core.calls, isEmpty);
  });

  test('asset references enforce the phase-one 64 MiB blob limit', () {
    final sha256 = List.filled(64, 'c').join();
    final atLimit = AuthoringAssetRef(
      sha256: sha256,
      byteLength: 64 * 1024 * 1024,
      logicalName: 'voice.ogg',
    );
    expect(atLimit.byteLength, 64 * 1024 * 1024);
    expect(
      () => AuthoringAssetRef(
        sha256: sha256,
        byteLength: 64 * 1024 * 1024 + 1,
        logicalName: 'voice.ogg',
      ),
      throwsFormatException,
    );
  });

  test('verify wrapper rejects a success response with extra fields', () async {
    final core = FakeGoreCoreFfiService(
      responses: {
        'authoring_store_verify_asset': {'ok': true, 'ignored': true},
      },
    );
    final asset = AuthoringAssetRef(
      sha256: List.filled(64, 'c').join(),
      byteLength: 1,
      logicalName: 'voice.ogg',
    );

    await expectLater(
      ModFfi(core).authoringStoreVerifyAsset(
        root: 'root',
        asset: asset,
        verification: AuthoringAssetVerification.full,
      ),
      throwsFormatException,
    );
  });
}
