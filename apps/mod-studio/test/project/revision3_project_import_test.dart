import 'dart:async';
import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_project_import.dart';

const _source = r'C:\portable\story-copy.goremod';
const _projectId = '11111111111111111111111111111111';
const _sha = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';

void main() {
  group('strict V2 inspection DTO', () {
    test('accepts only the closed restorable inspection response', () {
      final inspection = Revision3ProjectImportInspection.fromJson(
        _response(),
        expectedSource: _source,
      );

      expect(inspection.format, revision3ProjectImportFormatV2);
      expect(inspection.artifactKind, revision3ProjectImportArtifactKindV2);
      expect(inspection.restoreStatus, revision3ProjectImportRestoreStatusV2);
      expect(inspection.retrySafe, isTrue);
      expect(inspection.source, _source);
      expect(inspection.sourceLabel, 'story-copy.goremod');
      expect(inspection.projectId, _projectId);
      expect(inspection.projectRevision, 7);
      expect(inspection.head.canonicalJson, _headJson());
      expect(inspection.closure.storeObjects, 3);
      expect(inspection.closure.archiveEntries, 6);
    });

    test('rejects malformed fields, hex, IDs, head, and closure counts', () {
      final malformed = <Map<String, Object?>>[
        _response()..['unexpected'] = true,
        _response()
          ..['archive'] = <String, Object?>{
            'byte_len': 2048,
            'sha256': _sha.toUpperCase(),
          },
        _response()
          ..['archive'] = <String, Object?>{'byte_len': 4096, 'sha256': _sha},
        _response()..['project_id'] = 'AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA',
        _response()..['project_id'] = '00000000000000000000000000000000',
        _response()
          ..['head_json'] =
              '{ "store_format":1,"snapshot":{"byte_len":321,"sha256":"$_sha"}}',
        _response()
          ..['closure'] = <String, Object?>{
            'snapshot_objects': 1,
            'entity_objects': 1,
            'asset_objects': 1,
            'archive_entries': 7,
            'uncompressed_bytes': 4096,
          },
      ];

      for (final response in malformed) {
        expect(
          () => Revision3ProjectImportInspection.fromJson(
            response,
            expectedSource: _source,
          ),
          throwsFormatException,
        );
      }
    });

    test('rejects every bounded field above its explicit cap', () {
      final oversize = <Map<String, Object?>>[
        _response()
          ..['archive'] = <String, Object?>{
            'byte_len': revision3ProjectImportMaxArchiveBytes + 1,
            'sha256': _sha,
          },
        _response()
          ..['manifest'] = <String, Object?>{
            'relative_name': revision3ProjectImportManifestName,
            'byte_len': revision3ProjectImportMaxManifestBytes + 1,
            'sha256': _sha,
          },
        _response()
          ..['closure'] = <String, Object?>{
            'snapshot_objects': revision3ProjectImportMaxSnapshotObjects + 1,
            'entity_objects': 0,
            'asset_objects': 0,
            'archive_entries': revision3ProjectImportMaxSnapshotObjects + 4,
            'uncompressed_bytes': 4096,
          },
        _response()
          ..['closure'] = <String, Object?>{
            'snapshot_objects': revision3ProjectImportMaxSnapshotObjects,
            'entity_objects': revision3ProjectImportMaxEntityObjects,
            'asset_objects': revision3ProjectImportMaxAssetObjects + 1,
            'archive_entries': revision3ProjectImportMaxClosureObjects + 4,
            'uncompressed_bytes': 4096,
          },
        _response()
          ..['closure'] = <String, Object?>{
            'snapshot_objects': 1,
            'entity_objects': 1,
            'asset_objects': 1,
            'archive_entries': 6,
            'uncompressed_bytes':
                revision3ProjectImportMaxUncompressedBytes + 1,
          },
      ];

      for (final response in oversize) {
        expect(
          () => Revision3ProjectImportInspection.fromJson(
            response,
            expectedSource: _source,
          ),
          throwsFormatException,
        );
      }
      final longSource = '${'a' * revision3ProjectImportMaxSourceUtf8Bytes}x';
      expect(
        () => Revision3ProjectImportInspection.fromJson(
          _response(source: longSource),
          expectedSource: longSource,
        ),
        throwsFormatException,
      );
    });

    test('accepts an exact closure above the former 262144 aggregate cap', () {
      final response = _response()
        ..['closure'] = <String, Object?>{
          'snapshot_objects': 90000,
          'entity_objects': 90000,
          'asset_objects': 90000,
          'archive_entries': 270003,
          'uncompressed_bytes': 4096,
        };

      final inspection = Revision3ProjectImportInspection.fromJson(
        response,
        expectedSource: _source,
      );

      expect(inspection.closure.storeObjects, 270000);
      expect(inspection.closure.storeObjects, greaterThan(262144));
      expect(inspection.closure.archiveEntries, 270003);
    });

    test('requires exact inspect-only status pairing', () {
      final mismatches = <Map<String, Object?>>[
        _response()..['outcome'] = 'imported',
        _response()..['inspection_status'] = 'partial',
        _response()..['import_status'] = 'prepared',
        _response()..['project_mutation'] = 'performed',
        _response()..['game_mutation'] = 'performed',
        _response()..['publication_status'] = 'published',
        _response()..['retry_safe'] = false,
      ];

      for (final response in mismatches) {
        expect(
          () => Revision3ProjectImportInspection.fromJson(
            response,
            expectedSource: _source,
          ),
          throwsFormatException,
        );
      }
    });

    test('rejects V1-shaped and unknown success responses as malformed', () {
      final v1 = _response()
        ..['format'] = revision3ProjectImportUnsupportedFormatV1
        ..['artifact_kind'] = 'portable_snapshot_review_copy'
        ..['restore_status'] = 'not_supported';

      expect(
        () => Revision3ProjectImportInspection.fromJson(
          v1,
          expectedSource: _source,
        ),
        throwsFormatException,
      );
      expect(
        () => Revision3ProjectImportInspection.fromJson(
          _response()..['format'] = 'managed_revision3_exact_snapshot_v3',
          expectedSource: _source,
        ),
        throwsFormatException,
      );
    });

    test('binds native output to the exact source spelling', () {
      expect(
        () => Revision3ProjectImportInspection.fromJson(
          _response(source: r'c:\portable\story-copy.goremod'),
          expectedSource: _source,
        ),
        throwsFormatException,
      );
    });
  });

  group('inspect-only planning', () {
    test('picker cancellation is terminal without native inspection', () async {
      final owner = Object();
      var inspections = 0;
      final coordinator = Revision3ProjectImportInspectionCoordinator(
        readLifecycle: () =>
            Revision3ProjectImportLifecycle(owner: owner, generation: 0),
        pickSource: () async => null,
        inspect: (_) async {
          inspections++;
          return _response();
        },
      );
      addTearDown(coordinator.dispose);

      final result = await coordinator.plan();

      expect(result.outcome, Revision3ProjectImportPlanningOutcome.cancelled);
      expect(result.plan, isNull);
      expect(inspections, 0);
    });

    test('late inspection is suppressed after explicit cancellation', () async {
      final owner = Object();
      final entered = Completer<void>();
      final pending = Completer<Object?>();
      final coordinator = Revision3ProjectImportInspectionCoordinator(
        readLifecycle: () =>
            Revision3ProjectImportLifecycle(owner: owner, generation: 0),
        pickSource: () async => _source,
        inspect: (_) {
          entered.complete();
          return pending.future;
        },
      );
      addTearDown(coordinator.dispose);

      final operation = coordinator.plan();
      await entered.future;
      expect(coordinator.cancelPending(), isTrue);
      pending.complete(_response());

      final result = await operation;
      expect(result.outcome, Revision3ProjectImportPlanningOutcome.superseded);
      expect(result.plan, isNull);
      expect(coordinator.isBusy, isFalse);
    });

    test('lifecycle drift makes an otherwise valid result stale', () async {
      final owner = Object();
      var generation = 0;
      final entered = Completer<void>();
      final pending = Completer<Object?>();
      final coordinator = Revision3ProjectImportInspectionCoordinator(
        readLifecycle: () => Revision3ProjectImportLifecycle(
          owner: owner,
          generation: generation,
        ),
        pickSource: () async => _source,
        inspect: (_) {
          entered.complete();
          return pending.future;
        },
      );
      addTearDown(coordinator.dispose);

      final operation = coordinator.plan();
      await entered.future;
      generation++;
      pending.complete(_response());

      expect(
        (await operation).outcome,
        Revision3ProjectImportPlanningOutcome.stale,
      );
    });

    test('dispose suppresses late output and refuses later planning', () async {
      final owner = Object();
      final entered = Completer<void>();
      final pending = Completer<Object?>();
      final coordinator = Revision3ProjectImportInspectionCoordinator(
        readLifecycle: () =>
            Revision3ProjectImportLifecycle(owner: owner, generation: 0),
        pickSource: () async => _source,
        inspect: (_) {
          entered.complete();
          return pending.future;
        },
      );

      final operation = coordinator.plan();
      await entered.future;
      coordinator.dispose();
      pending.complete(_response());

      expect(
        (await operation).outcome,
        Revision3ProjectImportPlanningOutcome.superseded,
      );
      expect(
        (await coordinator.plan()).outcome,
        Revision3ProjectImportPlanningOutcome.unavailable,
      );
    });

    test(
      'a second invocation is refused while inspection is in flight',
      () async {
        final owner = Object();
        final entered = Completer<void>();
        final pending = Completer<Object?>();
        final coordinator = Revision3ProjectImportInspectionCoordinator(
          readLifecycle: () =>
              Revision3ProjectImportLifecycle(owner: owner, generation: 0),
          pickSource: () async => _source,
          inspect: (_) {
            entered.complete();
            return pending.future;
          },
        );
        addTearDown(coordinator.dispose);

        final first = coordinator.plan();
        await entered.future;
        final second = await coordinator.plan();
        expect(second.outcome, Revision3ProjectImportPlanningOutcome.busy);

        pending.complete(_response());
        final result = await first;
        expect(result.outcome, Revision3ProjectImportPlanningOutcome.inspected);
        expect(result.plan?.sourceLabel, 'story-copy.goremod');
      },
    );

    test('real native V1 error gets a distinct future-dialog outcome', () async {
      final owner = Object();
      const command = 'authoring_store_inspect_revision3_exact_snapshot_v2';
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          command: <String, Object?>{
            'ok': false,
            'error': <String, Object?>{
              'code': 'AUTHORING_REVISION3_IMPORT_UNSUPPORTED_REVIEW_COPY',
              'message':
                  'the selected snapshot is a V1 review copy and is not restorable',
            },
          },
        },
      );
      final ffi = ModFfi(core);
      final coordinator = Revision3ProjectImportInspectionCoordinator(
        readLifecycle: () =>
            Revision3ProjectImportLifecycle(owner: owner, generation: 0),
        pickSource: () async => _source,
        inspect: (source) =>
            ffi.authoringStoreInspectRevision3ExactSnapshotV2(source: source),
      );
      addTearDown(coordinator.dispose);

      expect(
        (await coordinator.plan()).outcome,
        Revision3ProjectImportPlanningOutcome.unsupportedFormat,
      );
      expect(core.calls.single.command, command);
    });

    test('real native source error stays a user-correctable outcome', () async {
      final owner = Object();
      const command = 'authoring_store_inspect_revision3_exact_snapshot_v2';
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          command: <String, Object?>{
            'ok': false,
            'error': <String, Object?>{
              'code': 'AUTHORING_REVISION3_IMPORT_SOURCE_INVALID',
              'message':
                  'the selected snapshot source could not be inspected as one safe regular file',
            },
          },
        },
      );
      final ffi = ModFfi(core);
      final coordinator = Revision3ProjectImportInspectionCoordinator(
        readLifecycle: () =>
            Revision3ProjectImportLifecycle(owner: owner, generation: 0),
        pickSource: () async => _source,
        inspect: (source) =>
            ffi.authoringStoreInspectRevision3ExactSnapshotV2(source: source),
      );
      addTearDown(coordinator.dispose);

      expect(
        (await coordinator.plan()).outcome,
        Revision3ProjectImportPlanningOutcome.invalidSource,
      );
      expect(core.calls.single.command, command);
    });

    test('normal results and failures expose only sanitized UI data', () async {
      const privateSource =
          r'C:\Users\Daniel\private-folder\Very Secret Project.goremod';
      expect(
        revision3ProjectImportSourceLabel(privateSource),
        'Very Secret Project.goremod',
      );
      expect(
        revision3ProjectImportSourceLabel(privateSource),
        isNot(contains('private-folder')),
      );

      final owner = Object();
      final coordinator = Revision3ProjectImportInspectionCoordinator(
        readLifecycle: () =>
            Revision3ProjectImportLifecycle(owner: owner, generation: 0),
        pickSource: () async => privateSource,
        inspect: (_) async => throw StateError(
          'failed while reading $privateSource and store/private/object',
        ),
      );
      addTearDown(coordinator.dispose);

      final result = await coordinator.plan();
      expect(
        result.outcome,
        Revision3ProjectImportPlanningOutcome.inspectionFailed,
      );
      expect(result.plan, isNull);
      expect(result.toString(), isNot(contains('Daniel')));

      try {
        Revision3ProjectImportInspection.fromJson(
          _response(source: privateSource)
            ..['archive'] = <String, Object?>{
              'byte_len': 1,
              'sha256': 'not hex',
            },
          expectedSource: privateSource,
        );
        fail('malformed response was accepted');
      } on FormatException catch (error) {
        expect(error.toString(), isNot(contains('private-folder')));
      }
    });
  });
}

Map<String, Object?> _response({String source = _source}) => <String, Object?>{
  'ok': true,
  'outcome': 'inspected_restorable_copy',
  'source': source,
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
  'head_json': _headJson(),
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

String _headJson() => jsonEncode(<String, Object?>{
  'store_format': 1,
  'snapshot': <String, Object?>{'byte_len': 321, 'sha256': _sha},
});
