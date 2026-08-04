import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_project_history.dart';
import 'package:gore_mod/project/revision3_project_history_page.dart';

void main() {
  testWidgets(
    'shows a friendly newest-first timeline without technical seals',
    (tester) async {
      final history = _history(currentRevision: 4, oldestRevision: 2);
      await tester.pumpWidget(
        _host(
          history: history,
          restore: (_, _) => throw StateError('not called'),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('Project history'), findsOneWidget);
      expect(find.text('Revision 4'), findsOneWidget);
      expect(find.text('Revision 3'), findsOneWidget);
      expect(find.text('Revision 2'), findsOneWidget);
      expect(find.text('Current'), findsOneWidget);
      expect(
        find.byKey(const Key('revision3-history-undo-last')),
        findsOneWidget,
      );
      expect(find.textContaining('aaaa'), findsNothing);
      expect(find.text('History starts at revision 2.'), findsOneWidget);
    },
  );

  testWidgets(
    'undo confirms and restores the immediate predecessor append-only',
    (tester) async {
      final history = _history(currentRevision: 7, oldestRevision: 5);
      final pending = Completer<Revision3ProjectHistoryRestorePublication>();
      Revision3ProjectHistoryEntry? selected;
      await tester.pumpWidget(
        _host(
          history: history,
          restore: (expected, target) {
            expect(identical(expected, history), isTrue);
            selected = target;
            return pending.future;
          },
        ),
      );
      await tester.pumpAndSettle();

      await tester.tap(find.byKey(const Key('revision3-history-undo-last')));
      await tester.pumpAndSettle();
      expect(
        find.text('Revision 6 will be saved as new revision 8.'),
        findsOneWidget,
      );

      await tester.tap(
        find.byKey(const Key('revision3-history-confirm-restore')),
      );
      await tester.pump();
      expect(selected?.projectRevision, 6);
      expect(
        find.byKey(const Key('revision3-history-restoring-barrier')),
        findsOneWidget,
      );

      pending.complete(
        Revision3ProjectHistoryRestorePublication(
          previousHead: history.basisHead,
          head: _head(8),
          projectId: history.projectId,
          previousProjectRevision: 7,
          projectRevision: 8,
          restoredFromHead: history.entries[1].head,
          restoredFromRevision: 6,
        ),
      );
      await tester.pumpAndSettle();
      expect(find.text('Revision 6 was restored.'), findsOneWidget);
    },
  );

  testWidgets(
    'dirty editor state leaves browsing available but disables restore',
    (tester) async {
      final history = _history(currentRevision: 3, oldestRevision: 2);
      await tester.pumpWidget(
        _host(
          history: history,
          canRestore: false,
          disabledReason: 'Finish or discard your open text edit first.',
          restore: (_, _) => throw StateError('not called'),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('Revision 2'), findsOneWidget);
      expect(
        find.text('Finish or discard your open text edit first.'),
        findsOneWidget,
      );
      final undo = tester.widget<FilledButton>(
        find.byKey(const Key('revision3-history-undo-last')),
      );
      expect(undo.onPressed, isNull);
    },
  );

  testWidgets('load failure can be retried', (tester) async {
    var attempts = 0;
    final history = _history(currentRevision: 1, oldestRevision: 1);
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Revision3ProjectHistoryPage(
            checkpointIdentity: 'one',
            load: () async {
              attempts++;
              if (attempts == 1) throw StateError('broken');
              return history;
            },
            restore: (_, _) => throw StateError('not called'),
            copy: _copy,
            canRestore: true,
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('Could not load history'), findsOneWidget);

    await tester.tap(find.text('Try again'));
    await tester.pumpAndSettle();
    expect(attempts, 2);
    expect(find.text('Revision 1'), findsOneWidget);
  });

  testWidgets('expired retention is stated without implying hidden paging', (
    tester,
  ) async {
    final history = _history(
      currentRevision: 300,
      oldestRevision: 299,
      historyTruncated: true,
    );
    await tester.pumpWidget(
      _host(
        history: history,
        restore: (_, _) => throw StateError('not called'),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Older versions have expired.'), findsOneWidget);
    expect(find.textContaining('outside this view'), findsNothing);
    expect(
      find.byKey(const Key('revision3-history-recording-start')),
      findsNothing,
    );
  });

  testWidgets('checkpoint change supersedes an in-flight older read', (
    tester,
  ) async {
    final first = Completer<Revision3ProjectHistorySnapshot>();
    final checkpoint = ValueNotifier<int>(1);
    addTearDown(checkpoint.dispose);
    final latest = _history(currentRevision: 2, oldestRevision: 2);
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ValueListenableBuilder<int>(
            valueListenable: checkpoint,
            builder: (context, revision, _) => Revision3ProjectHistoryPage(
              checkpointIdentity: revision,
              load: revision == 1 ? () => first.future : () async => latest,
              restore: (_, _) => throw StateError('not called'),
              copy: _copy,
              canRestore: true,
            ),
          ),
        ),
      ),
    );
    await tester.pump();

    checkpoint.value = 2;
    await tester.pumpAndSettle();
    expect(find.text('Revision 2'), findsOneWidget);

    first.complete(_history(currentRevision: 1, oldestRevision: 1));
    await tester.pumpAndSettle();
    expect(find.text('Revision 2'), findsOneWidget);
    expect(find.text('Revision 1'), findsNothing);
  });
}

Widget _host({
  required Revision3ProjectHistorySnapshot history,
  required Revision3ProjectHistoryRestorer restore,
  bool canRestore = true,
  String? disabledReason,
}) => MaterialApp(
  home: Scaffold(
    body: SizedBox(
      width: 1100,
      height: 760,
      child: Revision3ProjectHistoryPage(
        checkpointIdentity: history.basisHead.canonicalJson,
        load: () async => history,
        restore: restore,
        copy: _copy,
        canRestore: canRestore,
        restoreDisabledReason: disabledReason,
      ),
    ),
  ),
);

Revision3ProjectHistorySnapshot _history({
  required int currentRevision,
  required int oldestRevision,
  bool historyTruncated = false,
}) {
  const projectId = '11111111111111111111111111111111';
  final entries = <Revision3ProjectHistoryEntry>[
    for (var revision = currentRevision; revision >= oldestRevision; revision--)
      Revision3ProjectHistoryEntry(
        head: _head(revision),
        projectId: projectId,
        projectRevision: revision,
        isCurrent: revision == currentRevision,
      ),
  ];
  return Revision3ProjectHistorySnapshot(
    basisHead: entries.first.head,
    projectId: projectId,
    currentRevision: currentRevision,
    entries: entries,
    historyTruncated: historyTruncated,
  );
}

AuthoringWorkingHead _head(
  int revision,
) => AuthoringWorkingHead.fromCanonicalJson(
  '{"store_format":1,"snapshot":{"byte_len":$revision,"sha256":"${revision.toRadixString(16).padLeft(64, '0')}"}}',
);

const _copy = Revision3ProjectHistoryPageCopy(
  title: 'Project history',
  description: 'Return to an earlier project version without losing history.',
  projectOnlyBoundary: 'Only this project changes. The game and saves do not.',
  refresh: 'Refresh',
  loading: 'Loading history…',
  loadFailedTitle: 'Could not load history',
  retry: 'Try again',
  currentVersion: 'Current version',
  previousVersions: 'Previous versions',
  undoLastChange: 'Undo last change',
  restoreVersion: 'Restore this version',
  restoreDialogTitle: 'Restore project version?',
  restoreDialogBody: _restoreBody,
  restoreProjectOnlyBoundary:
      'The current version stays in history. The game and saves do not change.',
  cancel: 'Cancel',
  restore: 'Restore',
  restoring: 'Restoring project version…',
  restoreFailed: 'The project version could not be restored.',
  restoreSucceeded: _restoreSucceeded,
  noPreviousVersions: 'No previous versions have been recorded yet.',
  recordingStartsAt: _recordingStartsAt,
  olderVersionsExpired: 'Older versions have expired.',
  revisionLabel: _revisionLabel,
  currentBadge: 'Current',
);

String _restoreBody(int revision, int nextRevision) =>
    'Revision $revision will be saved as new revision $nextRevision.';
String _restoreSucceeded(int revision) => 'Revision $revision was restored.';
String _recordingStartsAt(int revision) =>
    'History starts at revision $revision.';
String _revisionLabel(int revision) => 'Revision $revision';
