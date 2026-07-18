import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_project_import.dart';

const _command = 'authoring_store_inspect_revision3_exact_snapshot_v2';
const _importCommand = 'authoring_store_import_revision3_exact_snapshot_v2';
const _source = r'C:\Portable\Exact "Restorable" Copy.goremod';
const _destination = r'C:\Projects\Restored Story';
const _projectId = '11111111111111111111111111111111';
const _sha = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
const _maxArchiveBytes = 70 * 1024 * 1024 * 1024;

void main() {
  test('required command handshake includes exact R3 snapshot inspection', () {
    expect(requiredStudioCoreCommands, contains(_command));
    expect(
      requiredStudioCoreCommands,
      orderedEquals(<String>[...requiredStudioCoreCommands]..sort()),
    );
  });

  test('required command handshake includes exact R3 snapshot import', () {
    expect(requiredStudioCoreCommands, contains(_importCommand));
    expect(
      requiredStudioCoreCommands,
      orderedEquals(<String>[...requiredStudioCoreCommands]..sort()),
    );
  });

  test('wrapper sends only the exact source spelling', () async {
    final response = _response();
    final core = FakeGoreCoreFfiService(
      responses: <String, Map<String, Object?>>{_command: response},
    );

    final result = await ModFfi(
      core,
    ).authoringStoreInspectRevision3ExactSnapshotV2(source: _source);

    expect(result, response);
    expect(core.calls.single.command, _command);
    expect(core.calls.single.payload.keys, <String>['source']);
    expect(core.calls.single.payload, <String, Object?>{'source': _source});
  });

  test('raw response feeds the strict project inspection DTO', () async {
    final core = FakeGoreCoreFfiService(
      responses: <String, Map<String, Object?>>{_command: _response()},
    );

    final raw = await ModFfi(
      core,
    ).authoringStoreInspectRevision3ExactSnapshotV2(source: _source);
    final inspection = Revision3ProjectImportInspection.fromJson(
      raw,
      expectedSource: _source,
    );

    expect(inspection.source, _source);
    expect(inspection.format, revision3ProjectImportFormatV2);
    expect(inspection.projectId, _projectId);
    expect(inspection.projectRevision, 7);
    expect(inspection.retrySafe, isTrue);
  });

  test('invalid source paths fail before any native call', () async {
    final invalid = <String>[
      '',
      'bad\u0000path',
      String.fromCharCode(0xd800),
      '${'é' * (16 * 1024)}x',
    ];
    for (final source in invalid) {
      final core = FakeGoreCoreFfiService(responses: const {});

      await expectLater(
        ModFfi(
          core,
        ).authoringStoreInspectRevision3ExactSnapshotV2(source: source),
        throwsArgumentError,
      );
      expect(core.calls, isEmpty, reason: 'invalid source reached native');
    }
  });

  test('malformed native error schemas remain transport failures', () async {
    final malformed = <Map<String, Object?>>[
      <String, Object?>{'ok': null},
      <String, Object?>{
        'ok': false,
        'error': <String, Object?>{
          'code': 'AUTHORING_REVISION3_IMPORT_SOURCE_INVALID',
          'message': 'invalid source',
          'path': _source,
        },
      },
      <String, Object?>{
        'ok': false,
        'error': <String, Object?>{
          'code': 'lowercase_code',
          'message': 'invalid source',
        },
      },
      <String, Object?>{
        'ok': false,
        'error': <String, Object?>{
          'code': 'AUTHORING_REVISION3_IMPORT_SOURCE_INVALID',
          'message': '   ',
        },
      },
    ];

    for (final response in malformed) {
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{_command: response},
      );
      await expectLater(
        ModFfi(
          core,
        ).authoringStoreInspectRevision3ExactSnapshotV2(source: _source),
        throwsA(
          isA<ModFfiException>()
              .having((error) => error.command, 'command', _command)
              .having(
                (error) => error.code,
                'code',
                ModFfiException.malformedNativeResponseCode,
              ),
        ),
      );
    }
  });

  test('import wrapper sends only the sealed three-field request', () async {
    final response = <String, Object?>{'ok': true, 'outcome': 'imported'};
    final core = FakeGoreCoreFfiService(
      responses: <String, Map<String, Object?>>{_importCommand: response},
    );

    final result = await ModFfi(core)
        .authoringStoreImportRevision3ExactSnapshotV2(
          source: _source,
          destination: _destination,
          expectedArchiveByteLength: 8192,
          expectedArchiveSha256: _sha,
        );

    expect(result, response);
    expect(core.calls.single.command, _importCommand);
    expect(
      core.calls.single.payload.keys,
      orderedEquals(<String>['source', 'destination', 'expected_archive']),
    );
    expect(core.calls.single.payload, <String, Object?>{
      'source': _source,
      'destination': _destination,
      'expected_archive': <String, Object?>{'byte_len': 8192, 'sha256': _sha},
    });
    final archive =
        core.calls.single.payload['expected_archive'] as Map<String, Object?>;
    expect(archive.keys, orderedEquals(<String>['byte_len', 'sha256']));
  });

  test(
    'import wrapper rejects invalid paths and archive seals preflight',
    () async {
      final invocations = <Future<Map<String, Object?>> Function(ModFfi)>[
        (ffi) => ffi.authoringStoreImportRevision3ExactSnapshotV2(
          source: '',
          destination: _destination,
          expectedArchiveByteLength: 8192,
          expectedArchiveSha256: _sha,
        ),
        (ffi) => ffi.authoringStoreImportRevision3ExactSnapshotV2(
          source: _source,
          destination: 'bad\u0000destination',
          expectedArchiveByteLength: 8192,
          expectedArchiveSha256: _sha,
        ),
        (ffi) => ffi.authoringStoreImportRevision3ExactSnapshotV2(
          source: _source,
          destination: _destination,
          expectedArchiveByteLength: 0,
          expectedArchiveSha256: _sha,
        ),
        (ffi) => ffi.authoringStoreImportRevision3ExactSnapshotV2(
          source: _source,
          destination: _destination,
          expectedArchiveByteLength: _maxArchiveBytes + 1,
          expectedArchiveSha256: _sha,
        ),
        (ffi) => ffi.authoringStoreImportRevision3ExactSnapshotV2(
          source: _source,
          destination: _destination,
          expectedArchiveByteLength: 8192,
          expectedArchiveSha256: 'A' * 64,
        ),
        (ffi) => ffi.authoringStoreImportRevision3ExactSnapshotV2(
          source: _source,
          destination: _destination,
          expectedArchiveByteLength: 8192,
          expectedArchiveSha256: 'a' * 63,
        ),
      ];

      for (final invoke in invocations) {
        final core = FakeGoreCoreFfiService(responses: const {});
        await expectLater(invoke(ModFfi(core)), throwsArgumentError);
        expect(core.calls, isEmpty, reason: 'invalid import reached native');
      }
    },
  );

  test(
    'import wrapper preserves native errors and sanitizes malformed ones',
    () async {
      final validErrorCore = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          _importCommand: <String, Object?>{
            'ok': false,
            'error': <String, Object?>{
              'code': 'AUTHORING_REVISION3_IMPORT_SOURCE_CHANGED',
              'message': 'the source no longer matches the inspected archive',
            },
          },
        },
      );

      await expectLater(
        ModFfi(validErrorCore).authoringStoreImportRevision3ExactSnapshotV2(
          source: _source,
          destination: _destination,
          expectedArchiveByteLength: 8192,
          expectedArchiveSha256: _sha,
        ),
        throwsA(
          isA<ModFfiException>()
              .having((error) => error.command, 'command', _importCommand)
              .having(
                (error) => error.code,
                'code',
                'AUTHORING_REVISION3_IMPORT_SOURCE_CHANGED',
              ),
        ),
      );

      final malformedErrorCore = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          _importCommand: <String, Object?>{
            'ok': false,
            'error': <String, Object?>{
              'code': 'AUTHORING_REVISION3_IMPORT_SOURCE_CHANGED',
              'message': 'private failure',
              'path': r'C:\Users\Daniel\private',
            },
          },
        },
      );

      await expectLater(
        ModFfi(malformedErrorCore).authoringStoreImportRevision3ExactSnapshotV2(
          source: _source,
          destination: _destination,
          expectedArchiveByteLength: 8192,
          expectedArchiveSha256: _sha,
        ),
        throwsA(
          isA<ModFfiException>()
              .having((error) => error.command, 'command', _importCommand)
              .having(
                (error) => error.code,
                'code',
                ModFfiException.malformedNativeResponseCode,
              )
              .having(
                (error) => error.toString(),
                'sanitized message',
                isNot(contains('Daniel')),
              ),
        ),
      );
    },
  );

  test(
    'import wrapper leaves successful receipt parsing to the project layer',
    () async {
      final raw = <String, Object?>{
        'ok': true,
        'outcome': 'future_terminal_shape',
      };
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{_importCommand: raw},
      );

      final result = await ModFfi(core)
          .authoringStoreImportRevision3ExactSnapshotV2(
            source: _source,
            destination: _destination,
            expectedArchiveByteLength: 8192,
            expectedArchiveSha256: _sha,
          );

      expect(result, raw);
    },
  );

  test(
    'malformed success remains raw until the strict DTO rejects it',
    () async {
      final malformed = <String, Object?>{
        'ok': true,
        'outcome': 'inspected_restorable_copy',
        'source': _source,
      };
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{_command: malformed},
      );

      final raw = await ModFfi(
        core,
      ).authoringStoreInspectRevision3ExactSnapshotV2(source: _source);

      expect(raw, malformed);
      expect(
        () => Revision3ProjectImportInspection.fromJson(
          raw,
          expectedSource: _source,
        ),
        throwsFormatException,
      );
    },
  );
}

Map<String, Object?> _response() => <String, Object?>{
  'ok': true,
  'outcome': 'inspected_restorable_copy',
  'source': _source,
  'format': revision3ProjectImportFormatV2,
  'artifact_kind': revision3ProjectImportArtifactKindV2,
  'restore_status': revision3ProjectImportRestoreStatusV2,
  'archive': <String, Object?>{'byte_len': 8192, 'sha256': _sha},
  'manifest': <String, Object?>{
    'relative_name': revision3ProjectImportManifestName,
    'byte_len': 512,
    'sha256': _sha,
  },
  'project_id': _projectId,
  'project_revision': 7,
  'head_json': jsonEncode(<String, Object?>{
    'store_format': 1,
    'snapshot': <String, Object?>{'byte_len': 321, 'sha256': _sha},
  }),
  'closure': <String, Object?>{
    'snapshot_objects': 1,
    'entity_objects': 1,
    'asset_objects': 1,
    'archive_entries': 6,
    'uncompressed_bytes': 4096,
  },
  'inspection_status': 'verified_exact',
  'import_status': 'not_performed',
  'project_mutation': 'not_performed',
  'game_mutation': 'not_performed',
  'save_mutation': 'not_performed',
  'build_status': 'not_performed',
  'deployment_status': 'not_performed',
  'runtime_status': 'runtime_unqualified',
  'publication_status': 'not_supported',
  'retry_safe': true,
};
