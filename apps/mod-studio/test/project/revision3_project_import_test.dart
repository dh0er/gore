import 'dart:async';
import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_project_import.dart';

const _source = r'C:\portable\story-copy.goremod';
const _destination = r'C:\projects\restored-story';
const _projectId = '11111111111111111111111111111111';
const _sha = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
const _otherSha =
    'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
const _cleanupWarningCode = 'AUTHORING_REVISION3_IMPORT_CLEANUP_WARNING';
const _cleanupWarningMessage =
    'the verified project was materialized, but private staging cleanup was incomplete';
const _uncertainWarningCode =
    'AUTHORING_REVISION3_IMPORT_PUBLICATION_UNCERTAIN';
const _uncertainWarningMessage =
    'project publication may have completed; do not retry automatically';

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

    test('rejects unknown snapshot formats as malformed', () {
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

    test('unrecognized native inspection errors stay generic', () async {
      final owner = Object();
      const command = 'authoring_store_inspect_revision3_exact_snapshot_v2';
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          command: <String, Object?>{
            'ok': false,
            'error': <String, Object?>{
              'code': 'AUTHORING_REVISION3_IMPORT_ARCHIVE_REJECTED',
              'message': 'the selected archive was rejected',
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
        Revision3ProjectImportPlanningOutcome.inspectionFailed,
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

  group('strict destination materialization contract', () {
    test(
      'request authority is exactly source, destination, and archive seal',
      () {
        final plan = _destinationPlan();

        expect(plan.source, _source);
        expect(plan.destination, _destination);
        expect(plan.destinationLabel, 'restored-story');
        expect(plan.request.toJson(), <String, Object?>{
          'source': _source,
          'destination': _destination,
          'expected_archive': <String, Object?>{
            'byte_len': 8192,
            'sha256': _sha,
          },
        });
        expect(plan.request.toJson().keys, <String>{
          'source',
          'destination',
          'expected_archive',
        });
      },
    );

    test('published terminals carry receipts and are never retry-safe', () {
      final plan = _destinationPlan();
      final imported = Revision3ProjectImportDestinationResult.fromJson(
        _destinationResponse(),
        expectedPlan: plan,
      );
      final cleanup = Revision3ProjectImportDestinationResult.fromJson(
        _destinationResponse(outcome: 'imported_with_cleanup_warning'),
        expectedPlan: plan,
      );

      expect(
        imported.outcome,
        Revision3ProjectImportMaterializationOutcome.imported,
      );
      expect(imported.receipt, isNotNull);
      expect(imported.receipt?.source, _source);
      expect(imported.receipt?.destination, _destination);
      expect(imported.receipt?.projectId, _projectId);
      expect(imported.receipt?.head.canonicalJson, _headJson());
      expect(imported.retrySafe, isFalse);
      expect(cleanup.hasCleanupWarning, isTrue);
      expect(cleanup.receipt, isNotNull);
      expect(cleanup.retrySafe, isFalse);
    });

    test('uncertainty has its own minimal schema and no receipt', () {
      final response = _destinationResponse(outcome: 'publication_uncertain');
      for (final forbidden in <String>{
        'archive',
        'manifest',
        'project_id',
        'project_revision',
        'head_json',
        'closure',
      }) {
        expect(response, isNot(contains(forbidden)));
      }

      final result = Revision3ProjectImportDestinationResult.fromJson(
        response,
        expectedPlan: _destinationPlan(),
      );

      expect(
        result.outcome,
        Revision3ProjectImportMaterializationOutcome.publicationUncertain,
      );
      expect(result.publicationIsUncertain, isTrue);
      expect(result.receipt, isNull);
      expect(result.retrySafe, isFalse);
    });

    test('binds every returned receipt field to the prior inspection', () {
      final mismatches = <Map<String, Object?>>[
        _destinationResponse()..['source'] = r'c:\portable\story-copy.goremod',
        _destinationResponse()..['destination'] = r'c:\projects\restored-story',
        _destinationResponse()
          ..['archive'] = <String, Object?>{
            'byte_len': 8192,
            'sha256': _otherSha,
          },
        _destinationResponse()
          ..['manifest'] = <String, Object?>{
            'relative_name': revision3ProjectImportManifestName,
            'byte_len': 512,
            'sha256': _otherSha,
          },
        _destinationResponse()
          ..['project_id'] = '22222222222222222222222222222222',
        _destinationResponse()..['project_revision'] = 8,
        _destinationResponse()..['head_json'] = _headJson(sha256: _otherSha),
        _destinationResponse()
          ..['closure'] = <String, Object?>{
            'snapshot_objects': 2,
            'entity_objects': 1,
            'asset_objects': 1,
            'archive_entries': 7,
            'uncompressed_bytes': 4096,
          },
      ];

      for (final response in mismatches) {
        expect(
          () => Revision3ProjectImportDestinationResult.fromJson(
            response,
            expectedPlan: _destinationPlan(),
          ),
          throwsFormatException,
        );
      }
    });

    test('rejects cross-wired schemas, status tuples, and warnings', () {
      final uncertainWithReceipt = _destinationResponse(
        outcome: 'publication_uncertain',
      )..['archive'] = <String, Object?>{'byte_len': 8192, 'sha256': _sha};
      final publishedWithoutReceipt = _destinationResponse()..remove('archive');
      final malformed = <Map<String, Object?>>[
        _destinationResponse()..['extra'] = true,
        uncertainWithReceipt,
        publishedWithoutReceipt,
        _destinationResponse()..['format'] = 'foreign_snapshot_format',
        _destinationResponse()..['import_status'] = 'prepared',
        _destinationResponse()..['project_mutation'] = 'performed',
        _destinationResponse()..['session_adoption'] = 'performed',
        _destinationResponse()..['game_mutation'] = 'performed',
        _destinationResponse()..['runtime_status'] = 'qualified',
        _destinationResponse()..['retry_safe'] = true,
        _destinationResponse()
          ..['publication_status'] = 'publication_uncertain',
        _destinationResponse()
          ..['warning'] = <String, Object?>{
            'code': _cleanupWarningCode,
            'message': _cleanupWarningMessage,
          },
        _destinationResponse(outcome: 'imported_with_cleanup_warning')
          ..['warning'] = null,
        _destinationResponse(outcome: 'publication_uncertain')
          ..['publication_status'] = 'published',
      ];

      for (final response in malformed) {
        expect(
          () => Revision3ProjectImportDestinationResult.fromJson(
            response,
            expectedPlan: _destinationPlan(),
          ),
          throwsFormatException,
        );
      }
    });

    test(
      'accepts drive-rooted and UNC project directories without a suffix',
      () {
        final inspection = _inspection();
        for (final destination in <String>[
          _destination,
          r'D:/mods/story project',
          r'\\studio-nas\mods\story-copy',
        ]) {
          final plan = Revision3ProjectImportDestinationPlan.fromInspection(
            inspection: inspection,
            destination: destination,
          );
          expect(plan.destination, destination);
          expect(plan.destination, isNot(endsWith('.goreproj')));
        }
      },
    );

    test('rejects relative, malformed, and source-equal destinations', () {
      final inspection = _inspection();
      for (final destination in <String>[
        '',
        'C:\\',
        '.',
        '..',
        'restored-story',
        r'.\restored-story',
        r'C:restored-story',
        r'\restored-story',
        r'\\server\share',
        r'\\server\\restored-story',
        r'\\?\C:\restored-story',
        r'\\.\C:\restored-story',
        r'C:\projects\..\restored-story',
        'C:\\projects\\restored-story\\',
        r'C:\projects\bad:name',
        r'C:\projects\story.',
        'C:\\projects\\story ',
        r'C:\projects\CON',
        r'C:\projects\com1.txt',
        r'C:\projects\COM9',
        'C:\\projects\\COM¹.txt',
        'C:\\projects\\COM²',
        'C:\\projects\\COM³',
        r'C:\projects\lpt1.txt',
        r'C:\projects\LPT9',
        'C:\\projects\\LPT¹.txt',
        'C:\\projects\\LPT²',
        'C:\\projects\\LPT³',
        _source,
      ]) {
        expect(
          () => Revision3ProjectImportDestinationPlan.fromInspection(
            inspection: inspection,
            destination: destination,
          ),
          throwsFormatException,
        );
      }
    });
  });

  group('destination materialization coordinator', () {
    test(
      'rejects owner and generation drift since inspection before picker',
      () async {
        for (final changeOwner in <bool>[false, true]) {
          var owner = Object();
          var generation = 0;
          final inspectionCoordinator =
              Revision3ProjectImportInspectionCoordinator(
                readLifecycle: () => Revision3ProjectImportLifecycle(
                  owner: owner,
                  generation: generation,
                ),
                pickSource: () async => _source,
                inspect: (_) async => _response(),
              );
          final inspected = await inspectionCoordinator.plan();
          expect(
            inspected.outcome,
            Revision3ProjectImportPlanningOutcome.inspected,
          );

          if (changeOwner) {
            owner = Object();
          } else {
            generation++;
          }
          var pickerCalls = 0;
          var nativeCalls = 0;
          final destinationCoordinator =
              Revision3ProjectImportDestinationCoordinator(
                readLifecycle: () => Revision3ProjectImportLifecycle(
                  owner: owner,
                  generation: generation,
                ),
                pickDestination: () async {
                  pickerCalls++;
                  return _destination;
                },
                importProject: (_) async {
                  nativeCalls++;
                  return _destinationResponse();
                },
              );

          final result = await destinationCoordinator.materialize(
            inspected.plan!,
          );

          expect(
            result.outcome,
            Revision3ProjectImportDestinationExecutionOutcome.stale,
          );
          expect(result.receipt, isNull);
          expect(pickerCalls, 0);
          expect(nativeCalls, 0);
          destinationCoordinator.dispose();
          inspectionCoordinator.dispose();
        }
      },
    );

    test('picker cancellation is terminal and never invokes native', () async {
      final owner = Object();
      var calls = 0;
      final coordinator = Revision3ProjectImportDestinationCoordinator(
        readLifecycle: () =>
            Revision3ProjectImportLifecycle(owner: owner, generation: 0),
        pickDestination: () async => null,
        importProject: (_) async {
          calls++;
          return _destinationResponse();
        },
      );
      addTearDown(coordinator.dispose);

      final result = await coordinator.materialize(
        await _inspectionPlan(owner),
      );

      expect(
        result.outcome,
        Revision3ProjectImportDestinationExecutionOutcome.cancelled,
      );
      expect(result.receipt, isNull);
      expect(result.retrySafe, isFalse);
      expect(calls, 0);
    });

    test(
      'passes only the sealed request and maps all native terminals',
      () async {
        for (final outcome in <String>[
          'imported',
          'imported_with_cleanup_warning',
          'publication_uncertain',
        ]) {
          final owner = Object();
          Revision3ProjectImportDestinationRequest? captured;
          var calls = 0;
          final coordinator = Revision3ProjectImportDestinationCoordinator(
            readLifecycle: () =>
                Revision3ProjectImportLifecycle(owner: owner, generation: 0),
            pickDestination: () async => _destination,
            importProject: (request) async {
              calls++;
              captured = request;
              return _destinationResponse(outcome: outcome);
            },
          );
          addTearDown(coordinator.dispose);

          final result = await coordinator.materialize(
            await _inspectionPlan(owner),
          );

          expect(calls, 1);
          expect(captured?.toJson().keys, <String>{
            'source',
            'destination',
            'expected_archive',
          });
          expect(result.retrySafe, isFalse);
          if (outcome == 'publication_uncertain') {
            expect(
              result.outcome,
              Revision3ProjectImportDestinationExecutionOutcome
                  .publicationUncertain,
            );
            expect(result.receipt, isNull);
          } else {
            expect(result.receipt, isNotNull);
          }
        }
      },
    );

    test('stale picker output never reaches native', () async {
      final owner = Object();
      var generation = 0;
      var calls = 0;
      final picker = Completer<String?>();
      final coordinator = Revision3ProjectImportDestinationCoordinator(
        readLifecycle: () => Revision3ProjectImportLifecycle(
          owner: owner,
          generation: generation,
        ),
        pickDestination: () => picker.future,
        importProject: (_) async {
          calls++;
          return _destinationResponse();
        },
      );
      addTearDown(coordinator.dispose);

      final operation = coordinator.materialize(await _inspectionPlan(owner));
      generation++;
      picker.complete(_destination);

      expect(
        (await operation).outcome,
        Revision3ProjectImportDestinationExecutionOutcome.stale,
      );
      expect(calls, 0);
    });

    test('cancel and dispose suppress late native receipts', () async {
      for (final dispose in <bool>[false, true]) {
        final owner = Object();
        final entered = Completer<void>();
        final pending = Completer<Object?>();
        final coordinator = Revision3ProjectImportDestinationCoordinator(
          readLifecycle: () =>
              Revision3ProjectImportLifecycle(owner: owner, generation: 0),
          pickDestination: () async => _destination,
          importProject: (_) {
            entered.complete();
            return pending.future;
          },
        );

        final operation = coordinator.materialize(await _inspectionPlan(owner));
        await entered.future;
        if (dispose) {
          coordinator.dispose();
        } else {
          expect(coordinator.cancelPending(), isTrue);
        }
        pending.complete(_destinationResponse());

        final result = await operation;
        expect(
          result.outcome,
          Revision3ProjectImportDestinationExecutionOutcome.superseded,
        );
        expect(result.receipt, isNull);
        expect(result.retrySafe, isFalse);
        coordinator.dispose();
      }
    });

    test('is single-flight while destination publication is pending', () async {
      final owner = Object();
      final entered = Completer<void>();
      final pending = Completer<Object?>();
      final coordinator = Revision3ProjectImportDestinationCoordinator(
        readLifecycle: () =>
            Revision3ProjectImportLifecycle(owner: owner, generation: 0),
        pickDestination: () async => _destination,
        importProject: (_) {
          entered.complete();
          return pending.future;
        },
      );
      addTearDown(coordinator.dispose);

      final first = coordinator.materialize(await _inspectionPlan(owner));
      await entered.future;
      final busy = await coordinator.materialize(await _inspectionPlan(owner));
      expect(
        busy.outcome,
        Revision3ProjectImportDestinationExecutionOutcome.busy,
      );
      expect(busy.retrySafe, isFalse);

      pending.complete(_destinationResponse());
      expect(
        (await first).outcome,
        Revision3ProjectImportDestinationExecutionOutcome.imported,
      );
    });

    test(
      'maps native failures once without retaining private details',
      () async {
        final cases =
            <String, Revision3ProjectImportDestinationExecutionOutcome>{
              'AUTHORING_REVISION3_IMPORT_DESTINATION_INVALID':
                  Revision3ProjectImportDestinationExecutionOutcome
                      .invalidDestination,
              'AUTHORING_REVISION3_IMPORT_SOURCE_CHANGED':
                  Revision3ProjectImportDestinationExecutionOutcome
                      .inspectionExpired,
              'AUTHORING_REVISION3_IMPORT_PLATFORM_UNSUPPORTED':
                  Revision3ProjectImportDestinationExecutionOutcome.unavailable,
              'INTERNAL': Revision3ProjectImportDestinationExecutionOutcome
                  .importFailed,
            };
        for (final entry in cases.entries) {
          final owner = Object();
          var calls = 0;
          final coordinator = Revision3ProjectImportDestinationCoordinator(
            readLifecycle: () =>
                Revision3ProjectImportLifecycle(owner: owner, generation: 0),
            pickDestination: () async => _destination,
            importProject: (_) async {
              calls++;
              throw ModFfiException(
                command: 'authoring_store_import_revision3_exact_snapshot_v2',
                code: entry.key,
                message:
                    r'private failure at C:\Users\Daniel\secret\restored-story',
              );
            },
          );
          addTearDown(coordinator.dispose);

          final result = await coordinator.materialize(
            await _inspectionPlan(owner),
          );

          expect(result.outcome, entry.value);
          expect(result.retrySafe, isFalse);
          expect(result.receipt, isNull);
          expect(result.toString(), isNot(contains('Daniel')));
          expect(calls, 1);
        }
      },
    );
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

Revision3ProjectImportInspection _inspection() =>
    Revision3ProjectImportInspection.fromJson(
      _response(),
      expectedSource: _source,
    );

Future<Revision3ProjectImportInspectionPlan> _inspectionPlan(
  Object owner, {
  int generation = 0,
}) async {
  final coordinator = Revision3ProjectImportInspectionCoordinator(
    readLifecycle: () =>
        Revision3ProjectImportLifecycle(owner: owner, generation: generation),
    pickSource: () async => _source,
    inspect: (_) async => _response(),
  );
  final result = await coordinator.plan();
  coordinator.dispose();
  return result.plan!;
}

Revision3ProjectImportDestinationPlan _destinationPlan() =>
    Revision3ProjectImportDestinationPlan.fromInspection(
      inspection: _inspection(),
      destination: _destination,
    );

Map<String, Object?> _destinationResponse({String outcome = 'imported'}) {
  final response = <String, Object?>{
    'ok': true,
    'outcome': outcome,
    'source': _source,
    'destination': _destination,
    'format': revision3ProjectImportFormatV2,
    'artifact_kind': revision3ProjectImportArtifactKindV2,
    'restore_status': revision3ProjectImportRestoreStatusV2,
    'inspection_status': 'verified_exact',
    'import_status': 'materialized',
    'project_mutation': 'materialized',
    'session_adoption': 'not_performed',
    'game_mutation': 'not_performed',
    'save_mutation': 'not_performed',
    'build_status': 'not_performed',
    'deployment_status': 'not_performed',
    'runtime_status': 'runtime_unqualified',
    'publication_status': switch (outcome) {
      'imported' => 'published',
      'imported_with_cleanup_warning' => 'published_with_cleanup_warning',
      'publication_uncertain' => 'publication_uncertain',
      _ => 'published',
    },
    'retry_safe': false,
    'warning': switch (outcome) {
      'imported' => null,
      'imported_with_cleanup_warning' => <String, Object?>{
        'code': _cleanupWarningCode,
        'message': _cleanupWarningMessage,
      },
      'publication_uncertain' => <String, Object?>{
        'code': _uncertainWarningCode,
        'message': _uncertainWarningMessage,
      },
      _ => null,
    },
  };
  if (outcome != 'publication_uncertain') {
    response.addAll(<String, Object?>{
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
    });
  }
  return response;
}

String _headJson({String sha256 = _sha}) => jsonEncode(<String, Object?>{
  'store_format': 1,
  'snapshot': <String, Object?>{'byte_len': 321, 'sha256': sha256},
});
