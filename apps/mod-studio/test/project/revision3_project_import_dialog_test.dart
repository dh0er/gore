import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/l10n/app_localizations.dart';
import 'package:gore_mod/l10n/app_localizations_en.dart';
import 'package:gore_mod/project/revision3_project_import.dart';
import 'package:gore_mod/project/revision3_project_import_dialog.dart';
import 'package:path/path.dart' as p;

const _source = r'C:\portable\story-copy.goremod';
const _projectId = '11111111111111111111111111111111';
const _sha = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
const _cleanupWarningCode = 'AUTHORING_REVISION3_IMPORT_CLEANUP_WARNING';
const _cleanupWarningMessage =
    'the verified project was materialized, but private staging cleanup was incomplete';
const _uncertainWarningCode =
    'AUTHORING_REVISION3_IMPORT_PUBLICATION_UNCERTAIN';
const _uncertainWarningMessage =
    'project publication may have completed; do not retry automatically';

void main() {
  test('folder and parent validators match the closed Windows grammar', () {
    final l10n = AppLocalizationsEn();
    final parent = Directory.systemTemp.createTempSync(
      'gore_import_dialog_validator_',
    );
    addTearDown(() => parent.deleteSync(recursive: true));
    final ordinaryFile = File(p.join(parent.path, 'not-a-parent'))
      ..writeAsStringSync('keep');

    expect(validateRevision3ProjectImportParent(parent.path, l10n), isNull);
    expect(validateRevision3ProjectImportParent('', l10n), isNotNull);
    expect(
      validateRevision3ProjectImportParent('relative-parent', l10n),
      isNotNull,
    );
    expect(
      validateRevision3ProjectImportParent(ordinaryFile.path, l10n),
      isNotNull,
    );

    for (final invalid in <String?>[
      null,
      '',
      '.',
      '..',
      ' leading',
      'trailing ',
      'trailing.',
      'two/parts',
      r'two\parts',
      'bad:name',
      'bad<name',
      'bad>name',
      'bad"name',
      'bad|name',
      'bad?name',
      'bad*name',
      'bad\u{0001}name',
      'bad\u{0085}name',
      'x' * 129,
    ]) {
      expect(
        validateRevision3ProjectImportFolderName(invalid, l10n),
        isNotNull,
        reason: 'invalid folder spelling: $invalid',
      );
    }

    for (final reserved in <String>[
      'CON',
      'prn.txt',
      'Aux.any',
      'nul',
      for (final prefix in <String>['COM', 'LPT'])
        for (var number = 1; number <= 9; number++) '$prefix$number',
      'com9.release',
      'LpT1.txt',
      'COM\u{00b9}',
      'com\u{00b2}.archive',
      'CoM\u{00b3}',
      'LPT\u{00b9}.copy',
      'lpt\u{00b2}',
      'LpT\u{00b3}.any',
    ]) {
      expect(
        validateRevision3ProjectImportFolderName(reserved, l10n),
        l10n.projectRestoreFolderNameReserved,
        reason: 'reserved folder spelling: $reserved',
      );
    }

    for (final allowed in <String>[
      'ordinary project',
      'COM0',
      'COM10.release',
      'LPT0',
      'LPT10',
      'M\u{00fc}nchen-Projekt',
    ]) {
      expect(
        validateRevision3ProjectImportFolderName(allowed, l10n),
        isNull,
        reason: 'allowed folder spelling: $allowed',
      );
    }
  });

  testWidgets(
    'shows the honest V2 authority and bounded verified metadata only',
    (tester) async {
      var inspectorCalls = 0;
      await _openDialog(
        tester,
        pickSource: () async => _source,
        inspect: (source) async {
          inspectorCalls++;
          expect(source, _source);
          return _inspectionResponse();
        },
      );

      expect(find.text('Restore into a new project folder'), findsOneWidget);
      expect(
        find.textContaining('verifies the complete archive'),
        findsOneWidget,
      );
      expect(
        find.textContaining('does not build, deploy, launch'),
        findsOneWidget,
      );
      expect(find.textContaining('game or any save'), findsOneWidget);

      await _inspect(tester);

      expect(inspectorCalls, 1);
      expect(
        find.byKey(const Key('revision3-project-import-no-source')),
        findsNothing,
      );
      expect(find.text('story-copy.goremod'), findsOneWidget);
      expect(find.text('7'), findsOneWidget);
      expect(find.text('8192'), findsOneWidget);
      expect(find.text('3'), findsOneWidget);
      expect(find.textContaining(r'C:\portable'), findsNothing);
      expect(find.textContaining(_projectId), findsNothing);
      expect(find.textContaining(_sha), findsNothing);
      expect(find.textContaining('complete and restorable'), findsOneWidget);
    },
  );

  final planningCases =
      <
        String,
        ({
          Revision3ProjectImportSourcePicker picker,
          Revision3ProjectImportNativeInspector inspector,
          String? visible,
          int nativeCalls,
        })
      >{
        'cancel': (
          picker: () async => null,
          inspector: (_) async => throw StateError('must not inspect'),
          visible: null,
          nativeCalls: 0,
        ),
        'V1 review copy': (
          picker: () async => _source,
          inspector: (_) async =>
              throw const Revision3ProjectImportUnsupportedFormatException(),
          visible: 'older review-only project copy',
          nativeCalls: 1,
        ),
        'invalid source': (
          picker: () async => 'bad\u{0000}source.goremod',
          inspector: (_) async => throw StateError('must not inspect'),
          visible: 'not a valid exact project backup',
          nativeCalls: 0,
        ),
        'inspection failure': (
          picker: () async => _source,
          inspector: (_) async => throw StateError(r'private C:\secret'),
          visible: 'could not be verified completely',
          nativeCalls: 1,
        ),
        'unavailable': (
          picker: () async => _source,
          inspector: (_) async => throw ModFfiException(
            command: 'authoring_store_inspect_revision3_exact_snapshot_v2',
            code: 'AUTHORING_REVISION3_IMPORT_PLATFORM_UNSUPPORTED',
            message: r'private C:\secret',
          ),
          visible: 'unavailable on this system',
          nativeCalls: 1,
        ),
      };
  for (final entry in planningCases.entries) {
    testWidgets('${entry.key} stays receipt-free and sanitized', (
      tester,
    ) async {
      var nativeCalls = 0;
      await _openDialog(
        tester,
        pickSource: entry.value.picker,
        inspect: (source) async {
          nativeCalls++;
          return entry.value.inspector(source);
        },
      );

      await _inspect(tester);

      expect(nativeCalls, entry.value.nativeCalls);
      expect(
        find.byKey(const Key('revision3-project-import-no-source')),
        findsOneWidget,
      );
      expect(_submit(tester).onPressed, isNull);
      if (entry.value.visible case final copy?) {
        expect(find.textContaining(copy), findsOneWidget);
      } else {
        expect(
          find.byKey(const Key('revision3-project-import-message')),
          findsNothing,
        );
      }
      expect(find.textContaining('private'), findsNothing);
      expect(find.textContaining('secret'), findsNothing);
    });
  }

  testWidgets('source reselection invalidates the prior plan and destination', (
    tester,
  ) async {
    final parent = _temporaryParent('source-reselection');
    var selections = 0;
    var inspections = 0;
    await _openDialog(
      tester,
      pickSource: () async => selections++ == 0 ? _source : null,
      inspect: (_) async {
        inspections++;
        return _inspectionResponse();
      },
      pickParent: () async => parent.path,
    );
    await _inspect(tester);
    await _chooseParent(tester);
    expect(find.text(parent.path), findsOneWidget);
    expect(_submit(tester).onPressed, isNotNull);

    await _inspect(tester);

    expect(inspections, 1);
    expect(
      find.byKey(const Key('revision3-project-import-no-source')),
      findsOneWidget,
    );
    expect(find.text(parent.path), findsNothing);
    expect(_submit(tester).onPressed, isNull);
  });

  testWidgets(
    'checks an absent destination and sends one exact sealed native request',
    (tester) async {
      final parent = _temporaryParent('exact-request');
      Revision3ProjectImportDestinationRequest? captured;
      var calls = 0;
      late final String destination;
      final probe = await _openDialog(
        tester,
        pickSource: () async => _source,
        inspect: (_) async => _inspectionResponse(),
        pickParent: () async => parent.path,
        importProject: (request) async {
          calls++;
          captured = request;
          destination = request.destination;
          expect(
            FileSystemEntity.typeSync(destination),
            FileSystemEntityType.notFound,
          );
          return _destinationResponse(destination: destination);
        },
      );
      await _inspectAndChooseParent(tester);

      final expectedDestination = p.normalize(
        p.join(parent.path, 'restored-project-r7'),
      );
      expect(
        find.byKey(const Key('revision3-project-import-destination-preview')),
        findsOneWidget,
      );
      expect(find.text(expectedDestination), findsOneWidget);
      await tester.tap(
        find.byKey(const Key('revision3-project-import-submit')),
      );
      await tester.pumpAndSettle();

      expect(calls, 1);
      expect(captured?.toJson(), <String, Object?>{
        'source': _source,
        'destination': expectedDestination,
        'expected_archive': <String, Object?>{'byte_len': 8192, 'sha256': _sha},
      });
      expect(probe.completed, isTrue);
      expect(probe.result?.hasCleanupWarning, isFalse);
      expect(probe.result?.receipt.destination, expectedDestination);
      expect(probe.result?.receipt.projectId, _projectId);
      expect(probe.result?.receipt.projectRevision, 7);
      expect(destination, expectedDestination);
    },
  );

  testWidgets('existing destination is rejected before native import', (
    tester,
  ) async {
    final parent = _temporaryParent('existing-destination');
    final existing = Directory(p.join(parent.path, 'restored-project-r7'))
      ..createSync();
    var calls = 0;
    await _openDialog(
      tester,
      pickSource: () async => _source,
      inspect: (_) async => _inspectionResponse(),
      pickParent: () async => parent.path,
      importProject: (_) async {
        calls++;
        throw StateError('must not import');
      },
    );
    await _inspectAndChooseParent(tester);

    await tester.tap(find.byKey(const Key('revision3-project-import-submit')));
    await tester.pumpAndSettle();

    expect(calls, 0);
    expect(existing.existsSync(), isTrue);
    expect(find.textContaining('already exists'), findsOneWidget);
    expect(_submit(tester).onPressed, isNotNull);
  });

  for (final outcome in <String>['imported', 'imported_with_cleanup_warning']) {
    testWidgets('$outcome returns exactly one confirmed dialog result', (
      tester,
    ) async {
      final parent = _temporaryParent(outcome);
      var calls = 0;
      final probe = await _openDialog(
        tester,
        pickSource: () async => _source,
        inspect: (_) async => _inspectionResponse(),
        pickParent: () async => parent.path,
        importProject: (request) async {
          calls++;
          return _destinationResponse(
            destination: request.destination,
            outcome: outcome,
          );
        },
      );
      await _inspectAndChooseParent(tester);

      await tester.tap(
        find.byKey(const Key('revision3-project-import-submit')),
      );
      await tester.pumpAndSettle();

      expect(calls, 1);
      expect(probe.completed, isTrue);
      expect(probe.result, isNotNull);
      expect(
        probe.result?.hasCleanupWarning,
        outcome == 'imported_with_cleanup_warning',
      );
      expect(probe.result?.receipt.source, _source);
      expect(
        find.byKey(const Key('revision3-project-import-dialog')),
        findsNothing,
      );
    });
  }

  testWidgets(
    'publication uncertainty is receipt-free terminal and cannot retry',
    (tester) async {
      final parent = _temporaryParent('uncertain');
      var calls = 0;
      final probe = await _openDialog(
        tester,
        pickSource: () async => _source,
        inspect: (_) async => _inspectionResponse(),
        pickParent: () async => parent.path,
        importProject: (request) async {
          calls++;
          return _destinationResponse(
            destination: request.destination,
            outcome: 'publication_uncertain',
          );
        },
      );
      await _inspectAndChooseParent(tester);

      await tester.tap(
        find.byKey(const Key('revision3-project-import-submit')),
      );
      await tester.pumpAndSettle();

      expect(calls, 1);
      expect(probe.completed, isFalse);
      expect(
        find.textContaining('cannot prove whether the project folder'),
        findsOneWidget,
      );
      expect(find.textContaining('Do not retry'), findsOneWidget);
      expect(
        find.byKey(const Key('revision3-project-import-submit')),
        findsNothing,
      );
      expect(
        tester
            .widget<OutlinedButton>(
              find.byKey(const Key('revision3-project-import-choose-source')),
            )
            .onPressed,
        isNull,
      );

      await tester.tap(find.byKey(const Key('revision3-project-import-close')));
      await tester.pumpAndSettle();
      expect(probe.completed, isTrue);
      expect(probe.result, isNull);
      expect(calls, 1);
    },
  );

  final importFailures = <String, ({Object Function() error, String copy})>{
    'source changed': (
      error: () => ModFfiException(
        command: 'authoring_store_import_revision3_exact_snapshot_v2',
        code: 'AUTHORING_REVISION3_IMPORT_SOURCE_CHANGED',
        message: r'private C:\secret',
      ),
      copy: 'changed after verification',
    ),
    'invalid destination': (
      error: () => ModFfiException(
        command: 'authoring_store_import_revision3_exact_snapshot_v2',
        code: 'AUTHORING_REVISION3_IMPORT_DESTINATION_INVALID',
        message: r'private C:\secret',
      ),
      copy: 'destination was rejected',
    ),
    'import failure': (
      error: () => StateError(r'private C:\secret'),
      copy: 'did not return a verified project receipt',
    ),
  };
  for (final entry in importFailures.entries) {
    testWidgets('${entry.key} is sanitized, terminal, and receipt-free', (
      tester,
    ) async {
      final parent = _temporaryParent(entry.key);
      var calls = 0;
      final probe = await _openDialog(
        tester,
        pickSource: () async => _source,
        inspect: (_) async => _inspectionResponse(),
        pickParent: () async => parent.path,
        importProject: (_) async {
          calls++;
          throw entry.value.error();
        },
      );
      await _inspectAndChooseParent(tester);

      await tester.tap(
        find.byKey(const Key('revision3-project-import-submit')),
      );
      await tester.pumpAndSettle();

      expect(calls, 1);
      expect(probe.completed, isFalse);
      expect(find.textContaining(entry.value.copy), findsOneWidget);
      expect(find.textContaining('private'), findsNothing);
      expect(find.textContaining('secret'), findsNothing);
      expect(
        find.byKey(const Key('revision3-project-import-submit')),
        findsNothing,
      );
      await tester.tap(find.byKey(const Key('revision3-project-import-close')));
      await tester.pumpAndSettle();
      expect(probe.completed, isTrue);
      expect(probe.result, isNull);
    });
  }

  testWidgets(
    'inspection can be closed or backed out while its Future is late',
    (tester) async {
      for (final useBack in <bool>[false, true]) {
        final inspection = Completer<Object?>();
        final entered = Completer<void>();
        final probe = await _openDialog(
          tester,
          pickSource: () async => _source,
          inspect: (_) {
            entered.complete();
            return inspection.future;
          },
        );
        await tester.tap(
          find.byKey(const Key('revision3-project-import-choose-source')),
        );
        await tester.pump();
        await entered.future;
        expect(
          tester
              .widget<TextButton>(
                find.byKey(const Key('revision3-project-import-close')),
              )
              .onPressed,
          isNotNull,
        );

        if (useBack) {
          await tester.binding.handlePopRoute();
        } else {
          await tester.tap(
            find.byKey(const Key('revision3-project-import-close')),
          );
        }
        await tester.pumpAndSettle();
        expect(probe.completed, isTrue);
        expect(probe.result, isNull);

        inspection.complete(_inspectionResponse());
        await tester.pump();
        expect(tester.takeException(), isNull);
      }
    },
  );

  testWidgets(
    'materialization is single-flight and disables close and back until done',
    (tester) async {
      final parent = _temporaryParent('single-flight');
      final pending = Completer<Object?>();
      final entered = Completer<void>();
      var calls = 0;
      late String destination;
      final probe = await _openDialog(
        tester,
        pickSource: () async => _source,
        inspect: (_) async => _inspectionResponse(),
        pickParent: () async => parent.path,
        importProject: (request) {
          calls++;
          destination = request.destination;
          entered.complete();
          return pending.future;
        },
      );
      await _inspectAndChooseParent(tester);

      await tester.tap(
        find.byKey(const Key('revision3-project-import-submit')),
      );
      await tester.pump();
      await entered.future;
      expect(calls, 1);
      expect(_submit(tester).onPressed, isNull);
      expect(
        tester
            .widget<TextButton>(
              find.byKey(const Key('revision3-project-import-close')),
            )
            .onPressed,
        isNull,
      );
      expect(
        find.byKey(const Key('revision3-project-import-materialize-progress')),
        findsOneWidget,
      );

      await tester.binding.handlePopRoute();
      await tester.pump();
      expect(
        find.byKey(const Key('revision3-project-import-dialog')),
        findsOneWidget,
      );
      expect(calls, 1);

      pending.complete(_destinationResponse(destination: destination));
      await tester.pumpAndSettle();
      expect(calls, 1);
      expect(probe.completed, isTrue);
      expect(probe.result, isNotNull);
    },
  );

  testWidgets('disposing the route suppresses a late native receipt', (
    tester,
  ) async {
    final parent = _temporaryParent('dispose-late');
    final pending = Completer<Object?>();
    final entered = Completer<void>();
    var calls = 0;
    late String destination;
    await _openDialog(
      tester,
      pickSource: () async => _source,
      inspect: (_) async => _inspectionResponse(),
      pickParent: () async => parent.path,
      importProject: (request) {
        calls++;
        destination = request.destination;
        entered.complete();
        return pending.future;
      },
    );
    await _inspectAndChooseParent(tester);
    await tester.tap(find.byKey(const Key('revision3-project-import-submit')));
    await tester.pump();
    await entered.future;

    await tester.pumpWidget(const SizedBox.shrink());
    pending.complete(_destinationResponse(destination: destination));
    await tester.pump();

    expect(calls, 1);
    expect(tester.takeException(), isNull);
    expect(
      find.byKey(const Key('revision3-project-import-dialog')),
      findsNothing,
    );
  });

  testWidgets(
    'opening starts behind one visible non-dismissible route and returns exactly',
    (tester) async {
      final pending = Completer<Object>();
      final entered = Completer<void>();
      final exactValue = Object();
      var calls = 0;
      var visibleWhenEntered = false;
      final probe = await _openOpeningProgress<Object>(
        tester,
        open: () {
          calls++;
          visibleWhenEntered = find
              .byKey(const Key('revision3-project-import-opening-dialog'))
              .evaluate()
              .isNotEmpty;
          entered.complete();
          return pending.future;
        },
      );
      await entered.future;
      await tester.pump();

      expect(calls, 1);
      expect(visibleWhenEntered, isTrue);
      expect(
        find.byKey(const Key('revision3-project-import-opening-dialog')),
        findsOneWidget,
      );
      expect(find.byType(CircularProgressIndicator), findsOneWidget);
      expect(
        tester.widget<ModalBarrier>(find.byType(ModalBarrier).last).dismissible,
        isFalse,
      );
      expect(probe.completed, isFalse);

      await tester.binding.handlePopRoute();
      await tester.pump();
      expect(
        find.byKey(const Key('revision3-project-import-opening-dialog')),
        findsOneWidget,
      );
      expect(calls, 1);

      pending.complete(exactValue);
      await tester.pumpAndSettle();

      expect(probe.completed, isTrue);
      expect(probe.value, same(exactValue));
      expect(probe.error, isNull);
      expect(
        find.byKey(const Key('revision3-project-import-opening-dialog')),
        findsNothing,
      );
    },
  );

  testWidgets(
    'opening rethrows the exact error without rendering its details',
    (tester) async {
      final release = Completer<void>();
      final entered = Completer<void>();
      final privateError = StateError(
        r'private opening failure at C:\Users\Daniel\secret\restored-project',
      );
      final probe = await _openOpeningProgress<Object>(
        tester,
        open: () async {
          entered.complete();
          await release.future;
          throw privateError;
        },
      );
      await entered.future;
      await tester.pump();

      expect(
        find.byKey(const Key('revision3-project-import-opening-dialog')),
        findsOneWidget,
      );
      expect(find.textContaining('private opening failure'), findsNothing);
      expect(find.textContaining(r'C:\Users\Daniel'), findsNothing);

      release.complete();
      await tester.pumpAndSettle();

      expect(probe.completed, isTrue);
      expect(probe.value, isNull);
      expect(probe.error, same(privateError));
      expect(probe.stackTrace, isNotNull);
      expect(tester.takeException(), isNull);
      expect(
        find.byKey(const Key('revision3-project-import-opening-dialog')),
        findsNothing,
      );
      expect(find.textContaining('private opening failure'), findsNothing);
      expect(find.textContaining(r'C:\Users\Daniel'), findsNothing);
    },
  );
}

final class _DialogProbe {
  Revision3ProjectImportDialogResult? result;
  bool completed = false;
}

final class _OpeningProbe<T> {
  T? value;
  Object? error;
  StackTrace? stackTrace;
  bool completed = false;
}

Future<_OpeningProbe<T>> _openOpeningProgress<T>(
  WidgetTester tester, {
  required Future<T> Function() open,
}) async {
  final probe = _OpeningProbe<T>();
  await tester.binding.setSurfaceSize(const Size(1200, 1000));
  addTearDown(() => tester.binding.setSurfaceSize(null));
  await tester.pumpWidget(
    MaterialApp(
      localizationsDelegates: const <LocalizationsDelegate<dynamic>>[
        AppLocalizations.delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
      ],
      supportedLocales: AppLocalizations.supportedLocales,
      home: Scaffold(
        body: Builder(
          builder: (context) => FilledButton(
            key: const Key('open-project-import-opening-progress'),
            onPressed: () async {
              try {
                probe.value =
                    await showRevision3ProjectImportOpeningProgress<T>(
                      context: context,
                      open: open,
                    );
              } catch (error, stackTrace) {
                probe.error = error;
                probe.stackTrace = stackTrace;
              } finally {
                probe.completed = true;
              }
            },
            child: const Text('Open'),
          ),
        ),
      ),
    ),
  );
  await tester.tap(
    find.byKey(const Key('open-project-import-opening-progress')),
  );
  await tester.pump();
  return probe;
}

Future<_DialogProbe> _openDialog(
  WidgetTester tester, {
  required Revision3ProjectImportSourcePicker pickSource,
  required Revision3ProjectImportNativeInspector inspect,
  Revision3ProjectImportParentDirectoryPicker? pickParent,
  Revision3ProjectImportNativeDestinationImporter? importProject,
}) async {
  final probe = _DialogProbe();
  await tester.binding.setSurfaceSize(const Size(1200, 1000));
  addTearDown(() => tester.binding.setSurfaceSize(null));
  await tester.pumpWidget(
    MaterialApp(
      localizationsDelegates: const <LocalizationsDelegate<dynamic>>[
        AppLocalizations.delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
      ],
      supportedLocales: AppLocalizations.supportedLocales,
      home: Scaffold(
        body: Builder(
          builder: (context) => FilledButton(
            key: const Key('open-project-import'),
            onPressed: () async {
              probe.result = await showRevision3ProjectImportDialog(
                context: context,
                pickSource: pickSource,
                inspect: inspect,
                pickExistingParentDirectory: pickParent ?? () async => null,
                importProject:
                    importProject ??
                    (_) async => throw StateError('must not import'),
              );
              probe.completed = true;
            },
            child: const Text('Open'),
          ),
        ),
      ),
    ),
  );
  await tester.tap(find.byKey(const Key('open-project-import')));
  await tester.pumpAndSettle();
  return probe;
}

Future<void> _inspect(WidgetTester tester) async {
  await tester.tap(
    find.byKey(const Key('revision3-project-import-choose-source')),
  );
  await tester.pumpAndSettle();
}

Future<void> _chooseParent(WidgetTester tester) async {
  await tester.tap(
    find.byKey(const Key('revision3-project-import-choose-parent')),
  );
  await tester.pumpAndSettle();
}

Future<void> _inspectAndChooseParent(WidgetTester tester) async {
  await _inspect(tester);
  await _chooseParent(tester);
}

FilledButton _submit(WidgetTester tester) => tester.widget<FilledButton>(
  find.byKey(const Key('revision3-project-import-submit')),
);

Directory _temporaryParent(String suffix) {
  final parent = Directory.systemTemp.createTempSync(
    'gore_import_dialog_${suffix.replaceAll(' ', '_')}_',
  );
  addTearDown(() {
    if (parent.existsSync()) parent.deleteSync(recursive: true);
  });
  return parent;
}

Map<String, Object?> _inspectionResponse() => <String, Object?>{
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

Map<String, Object?> _destinationResponse({
  required String destination,
  String outcome = 'imported',
}) {
  final response = <String, Object?>{
    'ok': true,
    'outcome': outcome,
    'source': _source,
    'destination': destination,
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

String _headJson() => jsonEncode(<String, Object?>{
  'store_format': 1,
  'snapshot': <String, Object?>{'byte_len': 321, 'sha256': _sha},
});
