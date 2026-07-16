import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/l10n/app_localizations.dart';
import 'package:gore_mod/project/current_project_controller.dart';
import 'package:gore_mod/project/revision3_project_export_dialog.dart';
import 'package:path/path.dart' as p;

const _projectId = '11111111111111111111111111111111';
const _revision = 7;
const _sha = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';

final _head = AuthoringWorkingHead.fromCanonicalJson(
  jsonEncode(<String, Object?>{
    'store_format': 1,
    'snapshot': <String, Object?>{'byte_len': 321, 'sha256': _sha},
  }),
);

void main() {
  testWidgets('validates a portable new .goremod file and never overwrites', (
    tester,
  ) async {
    final parent = Directory.systemTemp.createTempSync('gore_export_dialog_');
    addTearDown(() => parent.deleteSync(recursive: true));
    var calls = 0;
    await _openDialog(
      tester,
      pickParent: () async => parent.path,
      export: (output) async {
        calls++;
        return _result(output);
      },
    );

    expect(
      tester
          .widget<TextFormField>(
            find.byKey(const Key('revision3-project-export-file-name')),
          )
          .controller!
          .text,
      'project-copy-r7.goremod',
    );
    expect(_submit(tester).onPressed, isNull);
    await tester.enterText(
      find.byKey(const Key('revision3-project-export-file-name')),
      '../unsafe.goremod',
    );
    await tester.pump();
    expect(_submit(tester).onPressed, isNull);
    expect(find.textContaining('Start with a letter or digit'), findsOneWidget);

    const existingName = 'already-there.goremod';
    File(p.join(parent.path, existingName)).writeAsStringSync('keep');
    await tester.enterText(
      find.byKey(const Key('revision3-project-export-file-name')),
      existingName,
    );
    await _chooseParent(tester, parent.path);
    await tester.tap(find.byKey(const Key('revision3-project-export-submit')));
    await tester.pumpAndSettle();

    expect(calls, 0);
    expect(File(p.join(parent.path, existingName)).readAsStringSync(), 'keep');
    expect(find.textContaining('never overwritten'), findsOneWidget);
    expect(_submit(tester).onPressed, isNull);
  });

  testWidgets('picker cancellation is non-terminal and close cancels cleanly', (
    tester,
  ) async {
    await _openDialog(
      tester,
      pickParent: () async => null,
      export: (_) async => throw StateError('must not export'),
    );

    await tester.tap(
      find.byKey(const Key('revision3-project-export-choose-parent')),
    );
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key('revision3-project-export-dialog')),
      findsOneWidget,
    );
    expect(find.text('No destination folder selected'), findsOneWidget);
    await tester.tap(find.byKey(const Key('revision3-project-export-close')));
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key('revision3-project-export-dialog')),
      findsNothing,
    );
  });

  for (final code in const <String>{
    'AUTHORING_REVISION3_EXPORT_OUTPUT_EXISTS',
    'AUTHORING_REVISION3_EXPORT_OUTPUT_INVALID',
  }) {
    testWidgets(
      '$code requires a changed destination but does not seal retry',
      (tester) async {
        final parent = Directory.systemTemp.createTempSync(
          'gore_export_destination_retry_',
        );
        addTearDown(() => parent.deleteSync(recursive: true));
        var calls = 0;
        await _openDialog(
          tester,
          pickParent: () async => parent.path,
          export: (output) async {
            calls++;
            if (calls == 1) {
              throw Revision3ProjectExportFailedException(
                code: code,
                publicationMayExist: false,
                retryWithNewDestination: true,
              );
            }
            return _result(output);
          },
        );
        await _chooseParent(tester, parent.path);
        await tester.tap(
          find.byKey(const Key('revision3-project-export-submit')),
        );
        await tester.pumpAndSettle();

        expect(calls, 1);
        expect(
          find.textContaining(
            code.endsWith('OUTPUT_EXISTS')
                ? 'never overwritten'
                : 'Nothing was created',
          ),
          findsOneWidget,
        );
        expect(_submit(tester).onPressed, isNull);
        expect(
          tester
              .widget<TextFormField>(
                find.byKey(const Key('revision3-project-export-file-name')),
              )
              .enabled,
          isTrue,
        );
        expect(
          tester
              .widget<OutlinedButton>(
                find.byKey(const Key('revision3-project-export-choose-parent')),
              )
              .onPressed,
          isNotNull,
        );

        await _chooseParent(tester, parent.path);
        expect(calls, 1);
        expect(_submit(tester).onPressed, isNull);

        await tester.enterText(
          find.byKey(const Key('revision3-project-export-file-name')),
          'changed-project-copy.goremod',
        );
        await tester.pump();
        expect(_submit(tester).onPressed, isNotNull);

        await tester.enterText(
          find.byKey(const Key('revision3-project-export-file-name')),
          'project-copy-r7.goremod',
        );
        await tester.pump();
        expect(_submit(tester).onPressed, isNull);
        expect(
          find.textContaining(
            code.endsWith('OUTPUT_EXISTS')
                ? 'never overwritten'
                : 'Nothing was created',
          ),
          findsOneWidget,
        );

        await tester.enterText(
          find.byKey(const Key('revision3-project-export-file-name')),
          'changed-project-copy.goremod',
        );
        await tester.pump();
        expect(_submit(tester).onPressed, isNotNull);
        await tester.tap(
          find.byKey(const Key('revision3-project-export-submit')),
        );
        await tester.pumpAndSettle();

        expect(calls, 2);
        expect(
          find.byKey(const Key('revision3-project-export-published')),
          findsOneWidget,
        );
      },
    );
  }

  testWidgets('safe non-destination failure is terminal but output-absent', (
    tester,
  ) async {
    final parent = Directory.systemTemp.createTempSync(
      'gore_export_safe_failure_',
    );
    addTearDown(() => parent.deleteSync(recursive: true));
    await _openDialog(
      tester,
      pickParent: () async => parent.path,
      export: (_) async => throw const Revision3ProjectExportFailedException(
        code: 'AUTHORING_REVISION3_EXPORT_ARCHIVE_FAILED',
        publicationMayExist: false,
      ),
    );
    await _chooseParent(tester, parent.path);
    await tester.tap(find.byKey(const Key('revision3-project-export-submit')));
    await tester.pumpAndSettle();

    expect(
      find.textContaining('before the new local file was created'),
      findsOneWidget,
    );
    expect(find.textContaining('Nothing was created'), findsOneWidget);
    expect(find.textContaining('Do not retry'), findsNothing);
    expect(_submit(tester).onPressed, isNull);
  });

  testWidgets(
    'prevents double export and dismissal while the call is pending',
    (tester) async {
      final parent = Directory.systemTemp.createTempSync('gore_export_busy_');
      addTearDown(() => parent.deleteSync(recursive: true));
      final completion =
          Completer<AuthoringRevision3ExactSnapshotExportResult>();
      var calls = 0;
      late String output;
      await _openDialog(
        tester,
        pickParent: () async => parent.path,
        export: (received) {
          calls++;
          output = received;
          return completion.future;
        },
      );
      await _chooseParent(tester, parent.path);
      final submit = find.byKey(const Key('revision3-project-export-submit'));
      await tester.tap(submit);
      await tester.tap(submit);
      await tester.pump();

      expect(calls, 1);
      expect(
        find.byKey(const Key('revision3-project-export-progress')),
        findsOneWidget,
      );
      expect(
        tester
            .widget<TextButton>(
              find.byKey(const Key('revision3-project-export-close')),
            )
            .onPressed,
        isNull,
      );
      await tester.binding.handlePopRoute();
      await tester.pump();
      expect(
        find.byKey(const Key('revision3-project-export-dialog')),
        findsOneWidget,
      );

      completion.complete(_result(output));
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-project-export-published')),
        findsOneWidget,
      );
    },
  );

  for (final outcome in AuthoringRevision3ExactSnapshotExportOutcome.values) {
    testWidgets('renders sealed ${outcome.name} outcome without retry', (
      tester,
    ) async {
      final parent = Directory.systemTemp.createTempSync(
        'gore_export_${outcome.name}_',
      );
      addTearDown(() => parent.deleteSync(recursive: true));
      var calls = 0;
      await _openDialog(
        tester,
        pickParent: () async => parent.path,
        export: (output) async {
          calls++;
          return _result(output, outcome: outcome);
        },
      );
      await _chooseParent(tester, parent.path);
      await tester.tap(
        find.byKey(const Key('revision3-project-export-submit')),
      );
      await tester.pumpAndSettle();

      expect(calls, 1);
      expect(
        find.byKey(const Key('revision3-project-export-submit')),
        findsNothing,
      );
      expect(
        find.textContaining('current project remains open'),
        findsOneWidget,
      );
      expect(
        find.textContaining(RegExp(r'publish', caseSensitive: false)),
        findsNothing,
      );
      expect(find.textContaining('local file'), findsOneWidget);
      switch (outcome) {
        case AuthoringRevision3ExactSnapshotExportOutcome.exported:
          expect(
            find.byKey(const Key('revision3-project-export-published')),
            findsOneWidget,
          );
        case AuthoringRevision3ExactSnapshotExportOutcome
            .exportedWithCleanupWarning:
          expect(
            find.byKey(const Key('revision3-project-export-cleanup-warning')),
            findsOneWidget,
          );
          expect(find.textContaining('do not retry'), findsOneWidget);
        case AuthoringRevision3ExactSnapshotExportOutcome.publicationUncertain:
          expect(
            find.byKey(const Key('revision3-project-export-uncertain')),
            findsOneWidget,
          );
          expect(find.textContaining('Do not retry'), findsOneWidget);
      }
    });
  }

  final typedFailures = <String, Object Function()>{
    'stale': () => const Revision3ProjectExportStaleCheckpointException(),
    'requires reopen': () =>
        const Revision3ProjectExportRequiresReopenException(),
    'unsupported': () => const Revision3ProjectExportUnsupportedException(),
    'failed': () => const Revision3ProjectExportFailedException(),
  };
  for (final entry in typedFailures.entries) {
    testWidgets('${entry.key} failure is terminal and offers no retry', (
      tester,
    ) async {
      final parent = Directory.systemTemp.createTempSync(
        'gore_export_failure_',
      );
      addTearDown(() => parent.deleteSync(recursive: true));
      var calls = 0;
      await _openDialog(
        tester,
        pickParent: () async => parent.path,
        export: (_) async {
          calls++;
          throw entry.value();
        },
      );
      await _chooseParent(tester, parent.path);
      await tester.tap(
        find.byKey(const Key('revision3-project-export-submit')),
      );
      await tester.pumpAndSettle();

      expect(calls, 1);
      expect(_submit(tester).onPressed, isNull);
      expect(
        find.byKey(const Key('revision3-project-export-error')),
        findsOneWidget,
      );
      if (entry.key == 'stale' || entry.key == 'requires reopen') {
        expect(find.textContaining('No output was created'), findsOneWidget);
      }
    });
  }
}

Future<void> _openDialog(
  WidgetTester tester, {
  required Revision3ProjectExportParentDirectoryPicker pickParent,
  required Revision3ExactProjectExporter export,
}) async {
  await tester.binding.setSurfaceSize(const Size(1100, 900));
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
            key: const Key('open-project-export'),
            onPressed: () =>
                showDialog<AuthoringRevision3ExactSnapshotExportResult>(
                  context: context,
                  barrierDismissible: false,
                  builder: (_) => Revision3ProjectExportDialog(
                    projectRevision: _revision,
                    export: export,
                    pickExistingParentDirectory: pickParent,
                  ),
                ),
            child: const Text('Open'),
          ),
        ),
      ),
    ),
  );
  await tester.tap(find.byKey(const Key('open-project-export')));
  await tester.pumpAndSettle();
}

Future<void> _chooseParent(WidgetTester tester, String expected) async {
  await tester.tap(
    find.byKey(const Key('revision3-project-export-choose-parent')),
  );
  await tester.pumpAndSettle();
  expect(find.text(expected), findsOneWidget);
}

FilledButton _submit(WidgetTester tester) => tester.widget<FilledButton>(
  find.byKey(const Key('revision3-project-export-submit')),
);

AuthoringRevision3ExactSnapshotExportResult _result(
  String output, {
  AuthoringRevision3ExactSnapshotExportOutcome outcome =
      AuthoringRevision3ExactSnapshotExportOutcome.exported,
}) {
  final (wireOutcome, publicationStatus, warning) = switch (outcome) {
    AuthoringRevision3ExactSnapshotExportOutcome.exported => (
      'exported',
      'published',
      null,
    ),
    AuthoringRevision3ExactSnapshotExportOutcome.exportedWithCleanupWarning => (
      'exported_with_cleanup_warning',
      'published_with_cleanup_warning',
      <String, Object?>{
        'code': 'AUTHORING_REVISION3_EXPORT_CLEANUP_WARNING',
        'message':
            'the verified snapshot was published, but private staging cleanup was incomplete',
      },
    ),
    AuthoringRevision3ExactSnapshotExportOutcome.publicationUncertain => (
      'publication_uncertain',
      'publication_uncertain',
      <String, Object?>{
        'code': 'AUTHORING_REVISION3_EXPORT_PUBLICATION_UNCERTAIN',
        'message': 'publication may have completed; do not retry automatically',
      },
    ),
  };
  return AuthoringRevision3ExactSnapshotExportResult.fromJson(
    <String, Object?>{
      'ok': true,
      'outcome': wireOutcome,
      'format': 'managed_revision3_exact_snapshot_v1',
      'artifact_kind': 'portable_snapshot_review_copy',
      'restore_status': 'not_supported',
      'basis_head_json': _head.canonicalJson,
      'project_id': _projectId,
      'project_revision': _revision,
      'output': output,
      'archive': <String, Object?>{'byte_len': 300, 'sha256': _sha},
      'manifest': <String, Object?>{
        'relative_name': 'gore-export.json',
        'byte_len': 100,
        'sha256': 'b' * 64,
      },
      'closure': <String, Object?>{
        'snapshot_objects': 1,
        'entity_objects': 0,
        'asset_objects': 0,
        'archive_entries': 4,
        'uncompressed_bytes': 200,
      },
      'publication_status': publicationStatus,
      'retry_safe': false,
      'warning': warning,
      'project_mutation': 'not_performed',
      'game_mutation': 'not_performed',
      'save_mutation': 'not_performed',
      'build_status': 'not_performed',
      'deployment_status': 'not_performed',
      'runtime_status': 'runtime_unqualified',
    },
    expectedHead: _head,
    expectedOutput: output,
  );
}
