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

void main() {
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
