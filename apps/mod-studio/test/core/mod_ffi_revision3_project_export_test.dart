import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

const _command = 'authoring_store_export_revision3_exact_snapshot_v1';
const _commandV2 = 'authoring_store_export_revision3_exact_snapshot_v2';
const _root = r'C:\Projects\Exact "Review".goreproj';
const _output = r'C:\Exports\Exact "Review".goremod';
const _projectId = '00000000000000000000000000000003';
const _defaultWarning = Object();
const _maxV2ArchiveBytes = 70 * 1024 * 1024 * 1024;

String _repeat(String value, int count) =>
    List<String>.filled(count, value).join();

String _headJson([String byte = 'b']) => jsonEncode(<String, Object?>{
  'store_format': 1,
  'snapshot': <String, Object?>{'byte_len': 321, 'sha256': _repeat(byte, 64)},
});

Map<String, Object?> _response({
  String outcome = 'exported',
  String? publicationStatus,
  Object? warning = _defaultWarning,
}) {
  final resolvedStatus =
      publicationStatus ??
      switch (outcome) {
        'exported' => 'published',
        'exported_with_cleanup_warning' => 'published_with_cleanup_warning',
        'publication_uncertain' => 'publication_uncertain',
        _ => 'published',
      };
  final resolvedWarning = identical(warning, _defaultWarning)
      ? switch (outcome) {
          'exported' => null,
          'exported_with_cleanup_warning' => <String, Object?>{
            'code': 'AUTHORING_REVISION3_EXPORT_CLEANUP_WARNING',
            'message':
                'the verified snapshot was published, but private staging cleanup was incomplete',
          },
          'publication_uncertain' => <String, Object?>{
            'code': 'AUTHORING_REVISION3_EXPORT_PUBLICATION_UNCERTAIN',
            'message':
                'publication may have completed; do not retry automatically',
          },
          _ => null,
        }
      : warning;
  return <String, Object?>{
    'ok': true,
    'outcome': outcome,
    'format': 'managed_revision3_exact_snapshot_v1',
    'artifact_kind': 'portable_snapshot_review_copy',
    'restore_status': 'not_supported',
    'basis_head_json': _headJson(),
    'project_id': _projectId,
    'project_revision': 7,
    'output': _output,
    'archive': <String, Object?>{'byte_len': 4096, 'sha256': _repeat('c', 64)},
    'manifest': <String, Object?>{
      'relative_name': 'gore-export.json',
      'byte_len': 512,
      'sha256': _repeat('d', 64),
    },
    'closure': <String, Object?>{
      'snapshot_objects': 2,
      'entity_objects': 3,
      'asset_objects': 1,
      'archive_entries': 9,
      'uncompressed_bytes': 8192,
    },
    'publication_status': resolvedStatus,
    'retry_safe': false,
    'warning': resolvedWarning,
    'project_mutation': 'not_performed',
    'game_mutation': 'not_performed',
    'save_mutation': 'not_performed',
    'build_status': 'not_performed',
    'deployment_status': 'not_performed',
    'runtime_status': 'runtime_unqualified',
  };
}

Map<String, Object?> _copy(Map<String, Object?> value) =>
    (jsonDecode(jsonEncode(value)) as Map).cast<String, Object?>();

Map<String, Object?> _responseV2({
  String outcome = 'exported',
  String? publicationStatus,
  Object? warning = _defaultWarning,
}) {
  final response =
      _response(
          outcome: outcome,
          publicationStatus: publicationStatus,
          warning: warning,
        )
        ..['format'] = 'managed_revision3_exact_snapshot_v2'
        ..['artifact_kind'] = 'portable_snapshot_restorable_copy'
        ..['restore_status'] = 'supported';
  (response['archive'] as Map)['byte_len'] = 9216;
  return response;
}

AuthoringRevision3ExactSnapshotExportResult _parse(
  Map<String, Object?> response, {
  AuthoringWorkingHead? expectedHead,
  String expectedOutput = _output,
}) => AuthoringRevision3ExactSnapshotExportResult.fromJson(
  response,
  expectedHead:
      expectedHead ?? AuthoringWorkingHead.fromCanonicalJson(_headJson()),
  expectedOutput: expectedOutput,
);

AuthoringRevision3ExactSnapshotExportResultV2 _parseV2(
  Map<String, Object?> response, {
  AuthoringWorkingHead? expectedHead,
  String expectedOutput = _output,
}) => AuthoringRevision3ExactSnapshotExportResultV2.fromJson(
  response,
  expectedHead:
      expectedHead ?? AuthoringWorkingHead.fromCanonicalJson(_headJson()),
  expectedOutput: expectedOutput,
);

void main() {
  final head = AuthoringWorkingHead.fromCanonicalJson(_headJson());

  test('required command handshake includes exact R3 snapshot export', () {
    expect(requiredStudioCoreCommands, contains(_command));
    expect(
      requiredStudioCoreCommands,
      orderedEquals(<String>[...requiredStudioCoreCommands]..sort()),
    );
  });

  test('required command handshake includes restorable V2 export', () {
    expect(requiredStudioCoreCommands, contains(_commandV2));
  });

  test('wrapper sends only the exact three-field request', () async {
    final core = FakeGoreCoreFfiService(
      responses: <String, Map<String, Object?>>{_command: _response()},
    );

    final result = await ModFfi(core)
        .authoringStoreExportRevision3ExactSnapshotV1(
          root: _root,
          expectedHead: head,
          output: _output,
        );

    expect(
      result.outcome,
      AuthoringRevision3ExactSnapshotExportOutcome.exported,
    );
    expect(core.calls.single.command, _command);
    expect(core.calls.single.payload.keys, <String>[
      'expected_head_json',
      'output',
      'root',
    ]);
    expect(core.calls.single.payload, <String, Object?>{
      'expected_head_json': head.canonicalJson,
      'output': _output,
      'root': _root,
    });
  });

  test('strict DTO retains seals, closure, and all three terminals', () {
    final exported = _parse(_response());
    expect(
      exported.outcome,
      AuthoringRevision3ExactSnapshotExportOutcome.exported,
    );
    expect(
      exported.publicationStatus,
      AuthoringRevision3ExactSnapshotExportPublicationStatus.published,
    );
    expect(exported.hasCleanupWarning, isFalse);
    expect(exported.publicationIsUncertain, isFalse);
    expect(exported.warning, isNull);
    expect(exported.basisHead.canonicalJson, head.canonicalJson);
    expect(exported.projectId, _projectId);
    expect(exported.projectRevision, 7);
    expect(exported.output, _output);
    expect(exported.archive.byteLength, 4096);
    expect(exported.archive.sha256, _repeat('c', 64));
    expect(exported.manifest.relativeName, 'gore-export.json');
    expect(exported.manifest.byteLength, 512);
    expect(exported.closure.snapshotObjects, 2);
    expect(exported.closure.entityObjects, 3);
    expect(exported.closure.assetObjects, 1);
    expect(exported.closure.archiveEntries, 9);
    expect(exported.closure.uncompressedBytes, 8192);

    final cleanup = _parse(_response(outcome: 'exported_with_cleanup_warning'));
    expect(
      cleanup.outcome,
      AuthoringRevision3ExactSnapshotExportOutcome.exportedWithCleanupWarning,
    );
    expect(cleanup.hasCleanupWarning, isTrue);
    expect(cleanup.publicationIsUncertain, isFalse);
    expect(
      cleanup.publicationStatus,
      AuthoringRevision3ExactSnapshotExportPublicationStatus
          .publishedWithCleanupWarning,
    );
    expect(cleanup.warning!.code, 'AUTHORING_REVISION3_EXPORT_CLEANUP_WARNING');

    final uncertain = _parse(_response(outcome: 'publication_uncertain'));
    expect(
      uncertain.outcome,
      AuthoringRevision3ExactSnapshotExportOutcome.publicationUncertain,
    );
    expect(uncertain.publicationIsUncertain, isTrue);
    expect(uncertain.hasCleanupWarning, isFalse);
    expect(
      uncertain.publicationStatus,
      AuthoringRevision3ExactSnapshotExportPublicationStatus
          .publicationUncertain,
    );
    expect(
      uncertain.warning!.message,
      'publication may have completed; do not retry automatically',
    );
  });

  test('strict DTO binds the exact head and caller output spelling', () {
    expect(
      () => _parse(_response()..['basis_head_json'] = _headJson('a')),
      throwsFormatException,
    );
    expect(
      () => _parse(_response()..['output'] = r'C:\Exports\Other.goremod'),
      throwsFormatException,
    );
    expect(
      () => _parse(_response(), expectedOutput: _output.toLowerCase()),
      throwsFormatException,
    );
  });

  test('strict DTO rejects malformed seals and inconsistent closure', () {
    final mutations = <void Function(Map<String, Object?>)>[
      (response) => (response['archive'] as Map)['byte_len'] = 0,
      (response) => (response['archive'] as Map)['sha256'] = _repeat('C', 64),
      (response) => (response['archive'] as Map)['path'] = _output,
      (response) =>
          (response['manifest'] as Map)['relative_name'] = 'manifest.json',
      (response) =>
          (response['manifest'] as Map)['byte_len'] = 128 * 1024 * 1024 + 1,
      (response) => (response['closure'] as Map)['snapshot_objects'] = 0,
      (response) => (response['closure'] as Map)['entity_objects'] = 100001,
      (response) => (response['closure'] as Map)['archive_entries'] = 8,
      (response) => (response['closure'] as Map)['uncompressed_bytes'] = 511,
      (response) => (response['closure'] as Map)['future_count'] = 0,
    ];
    for (final mutate in mutations) {
      final response = _copy(_response());
      mutate(response);
      expect(() => _parse(response), throwsFormatException);
    }
  });

  test('strict DTO rejects widened authority and terminal disagreement', () {
    final mutations = <Map<String, Object?> Function()>[
      () => _response()..remove('restore_status'),
      () => _response()..['future_authority'] = true,
      () => _response()..['format'] = 'managed_revision3_exact_snapshot_v2',
      () => _response()..['artifact_kind'] = 'restorable_backup',
      () => _response()..['restore_status'] = 'supported',
      () => _response()..['project_mutation'] = 'performed',
      () => _response()..['game_mutation'] = 'performed',
      () => _response()..['save_mutation'] = 'performed',
      () => _response()..['build_status'] = 'built',
      () => _response()..['deployment_status'] = 'performed',
      () => _response()..['runtime_status'] = 'runtime_qualified',
      () => _response()..['retry_safe'] = true,
      () => _response()..['publication_status'] = 'publication_uncertain',
      () => _response()
        ..['warning'] = <String, Object?>{
          'code': 'AUTHORING_REVISION3_EXPORT_CLEANUP_WARNING',
          'message':
              'the verified snapshot was published, but private staging cleanup was incomplete',
        },
      () => _response(outcome: 'exported_with_cleanup_warning', warning: null),
      () => _response(
        outcome: 'publication_uncertain',
        warning: <String, Object?>{
          'code': 'AUTHORING_REVISION3_EXPORT_PUBLICATION_UNCERTAIN',
          'message': 'changed',
        },
      ),
    ];
    for (final mutate in mutations) {
      expect(() => _parse(mutate()), throwsFormatException);
    }
  });

  test('wrapper turns malformed native success into transport error', () async {
    final malformed = _response()..['retry_safe'] = true;
    final core = FakeGoreCoreFfiService(
      responses: <String, Map<String, Object?>>{_command: malformed},
    );

    await expectLater(
      ModFfi(core).authoringStoreExportRevision3ExactSnapshotV1(
        root: _root,
        expectedHead: head,
        output: _output,
      ),
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
  });

  test('wrapper rejects unsafe paths before native invocation', () async {
    for (final path in <String>['', 'bad\u0000path']) {
      final core = FakeGoreCoreFfiService(responses: const {});
      await expectLater(
        ModFfi(core).authoringStoreExportRevision3ExactSnapshotV1(
          root: path,
          expectedHead: head,
          output: _output,
        ),
        throwsArgumentError,
      );
      expect(core.calls, isEmpty);
    }
  });

  test('V2 wrapper sends only the exact three-field request', () async {
    final core = FakeGoreCoreFfiService(
      responses: <String, Map<String, Object?>>{_commandV2: _responseV2()},
    );

    final result = await ModFfi(core)
        .authoringStoreExportRevision3ExactSnapshotV2(
          root: _root,
          expectedHead: head,
          output: _output,
        );

    expect(
      result.outcome,
      AuthoringRevision3ExactSnapshotExportOutcome.exported,
    );
    expect(result.isRestorableProjectCopy, isTrue);
    expect(core.calls.single.command, _commandV2);
    expect(core.calls.single.payload.keys, <String>[
      'expected_head_json',
      'output',
      'root',
    ]);
    expect(core.calls.single.payload, <String, Object?>{
      'expected_head_json': head.canonicalJson,
      'output': _output,
      'root': _root,
    });
  });

  test('V2 DTO retains seals, closure, and all three terminals', () {
    final exported = _parseV2(_responseV2());
    expect(
      exported.outcome,
      AuthoringRevision3ExactSnapshotExportOutcome.exported,
    );
    expect(
      exported.publicationStatus,
      AuthoringRevision3ExactSnapshotExportPublicationStatus.published,
    );
    expect(exported.hasCleanupWarning, isFalse);
    expect(exported.publicationIsUncertain, isFalse);
    expect(exported.warning, isNull);
    expect(exported.basisHead.canonicalJson, head.canonicalJson);
    expect(exported.projectId, _projectId);
    expect(exported.projectRevision, 7);
    expect(exported.output, _output);
    expect(exported.archive.byteLength, 9216);
    expect(exported.archive.sha256, _repeat('c', 64));
    expect(exported.manifest.relativeName, 'gore-export.json');
    expect(exported.manifest.byteLength, 512);
    expect(exported.closure.snapshotObjects, 2);
    expect(exported.closure.entityObjects, 3);
    expect(exported.closure.assetObjects, 1);
    expect(exported.closure.archiveEntries, 9);
    expect(exported.closure.uncompressedBytes, 8192);

    final cleanup = _parseV2(
      _responseV2(outcome: 'exported_with_cleanup_warning'),
    );
    expect(
      cleanup.outcome,
      AuthoringRevision3ExactSnapshotExportOutcome.exportedWithCleanupWarning,
    );
    expect(cleanup.hasCleanupWarning, isTrue);
    expect(cleanup.publicationIsUncertain, isFalse);
    expect(
      cleanup.publicationStatus,
      AuthoringRevision3ExactSnapshotExportPublicationStatus
          .publishedWithCleanupWarning,
    );
    expect(cleanup.warning!.code, 'AUTHORING_REVISION3_EXPORT_CLEANUP_WARNING');

    final uncertain = _parseV2(_responseV2(outcome: 'publication_uncertain'));
    expect(
      uncertain.outcome,
      AuthoringRevision3ExactSnapshotExportOutcome.publicationUncertain,
    );
    expect(uncertain.publicationIsUncertain, isTrue);
    expect(uncertain.hasCleanupWarning, isFalse);
    expect(
      uncertain.publicationStatus,
      AuthoringRevision3ExactSnapshotExportPublicationStatus
          .publicationUncertain,
    );
    expect(
      uncertain.warning!.message,
      'publication may have completed; do not retry automatically',
    );
  });

  test('V1 and V2 DTOs reject every cross-wired authority tuple', () {
    expect(() => _parse(_responseV2()), throwsFormatException);
    expect(() => _parseV2(_response()), throwsFormatException);

    for (final field in <String>['format', 'artifact_kind', 'restore_status']) {
      final mixedV1 = _response();
      mixedV1[field] = _responseV2()[field];
      expect(() => _parse(mixedV1), throwsFormatException);

      final mixedV2 = _responseV2();
      mixedV2[field] = _response()[field];
      expect(() => _parseV2(mixedV2), throwsFormatException);
    }
  });

  test('V2 DTO enforces native closure and 70 GiB byte caps', () {
    final atLimit = _responseV2();
    (atLimit['archive'] as Map)['byte_len'] = _maxV2ArchiveBytes;
    (atLimit['closure'] as Map)['uncompressed_bytes'] = _maxV2ArchiveBytes - 1;
    expect(_parseV2(atLimit).archive.byteLength, _maxV2ArchiveBytes);

    final mutations = <void Function(Map<String, Object?>)>[
      (response) =>
          (response['archive'] as Map)['byte_len'] = _maxV2ArchiveBytes + 1,
      (response) => (response['closure'] as Map)['uncompressed_bytes'] =
          _maxV2ArchiveBytes + 1,
      (response) => (response['closure'] as Map)['snapshot_objects'] = 100001,
      (response) => (response['closure'] as Map)['entity_objects'] = 100001,
      (response) => (response['closure'] as Map)['asset_objects'] = 100001,
      (response) => (response['closure'] as Map)['archive_entries'] = 300004,
      (response) => (response['archive'] as Map)['byte_len'] =
          (response['closure'] as Map)['uncompressed_bytes'],
      (response) => response['project_id'] = '00000000000000000000000000000000',
    ];
    for (final mutate in mutations) {
      final response = _copy(_responseV2());
      mutate(response);
      expect(() => _parseV2(response), throwsFormatException);
    }
  });

  test('V2 DTO binds head/output and rejects terminal disagreement', () {
    expect(
      () => _parseV2(_responseV2()..['basis_head_json'] = _headJson('a')),
      throwsFormatException,
    );
    expect(
      () => _parseV2(_responseV2()..['output'] = r'C:\Exports\Other.goremod'),
      throwsFormatException,
    );
    final mutations = <Map<String, Object?> Function()>[
      () => _responseV2()..['retry_safe'] = true,
      () => _responseV2()..['project_mutation'] = 'performed',
      () => _responseV2()..['publication_status'] = 'publication_uncertain',
      () => _responseV2()
        ..['warning'] = <String, Object?>{
          'code': 'AUTHORING_REVISION3_EXPORT_CLEANUP_WARNING',
          'message':
              'the verified snapshot was published, but private staging cleanup was incomplete',
        },
      () =>
          _responseV2(outcome: 'exported_with_cleanup_warning', warning: null),
      () => _responseV2(
        outcome: 'publication_uncertain',
        warning: <String, Object?>{
          'code': 'AUTHORING_REVISION3_EXPORT_PUBLICATION_UNCERTAIN',
          'message': 'changed',
        },
      ),
    ];
    for (final mutate in mutations) {
      expect(() => _parseV2(mutate()), throwsFormatException);
    }
  });

  test('V2 wrapper maps malformed success and rejects unsafe paths', () async {
    final malformed = _responseV2()..['restore_status'] = 'not_supported';
    final core = FakeGoreCoreFfiService(
      responses: <String, Map<String, Object?>>{_commandV2: malformed},
    );
    await expectLater(
      ModFfi(core).authoringStoreExportRevision3ExactSnapshotV2(
        root: _root,
        expectedHead: head,
        output: _output,
      ),
      throwsA(
        isA<ModFfiException>()
            .having((error) => error.command, 'command', _commandV2)
            .having(
              (error) => error.code,
              'code',
              ModFfiException.malformedNativeResponseCode,
            ),
      ),
    );

    for (final path in <String>['', 'bad\u0000path']) {
      final noCall = FakeGoreCoreFfiService(responses: const {});
      await expectLater(
        ModFfi(noCall).authoringStoreExportRevision3ExactSnapshotV2(
          root: path,
          expectedHead: head,
          output: _output,
        ),
        throwsArgumentError,
      );
      expect(noCall.calls, isEmpty);
    }
  });
}
