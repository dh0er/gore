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
      final malformedResponses = <Map<String, Object?>>[
        const {},
        const {'ok': 'false'},
        const {'ok': false},
        const {'ok': false, 'error': 'bad'},
        const {'ok': false, 'error': <String, Object?>{}},
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
}
