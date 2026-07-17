import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_voice_folder_authoring.dart';
import 'package:gore_mod/project/revision3_voice_folder_import_dialog.dart';

void main() {
  testWidgets(
    'wide review hides technical authority and applies one complete plan',
    (tester) async {
      await _useSurface(tester, const Size(1300, 1000));
      const absoluteFolder = r'C:\Private\Daniel\Secret Voice';
      const projectId = 'entity:project-secret';
      const projectHead = 'sha256:deadbeef-head';
      const checkpoint = 'checkpoint:deadbeef';
      final plan = _plan(
        projectId: projectId,
        projectHead: projectHead,
        checkpointToken: checkpoint,
        planToken: 'plan:deadbeef',
        ignoredEntryCount: 1,
        rows: [_readyRow(0), _alreadyPresentRow(1)],
      );
      final applyCompletion =
          Completer<Revision3VoiceFolderImportPublication>();
      Revision3VoiceFolderPlanRequest? capturedRequest;
      Revision3VoiceFolderImportPlan? capturedPlan;
      var applyCalls = 0;
      final service = Revision3VoiceFolderAuthoringService(
        planFolder: (request) async {
          capturedRequest = request;
          return plan;
        },
        applyPlan: ({required plan}) {
          applyCalls++;
          capturedPlan = plan;
          return applyCompletion.future;
        },
      );
      await _openRoute(
        tester,
        service: service,
        projectId: projectId,
        projectHead: projectHead,
        checkpointToken: checkpoint,
        picker: () async => absoluteFolder,
      );

      await tester.tap(find.byKey(const Key('revision3-voice-folder-pick')));
      await tester.pumpAndSettle();
      expect(find.text('Recording folder selected'), findsOneWidget);
      expect(find.text('Secret Voice'), findsNothing);
      expect(find.textContaining(r'C:\Private'), findsNothing);

      await tester.tap(find.byKey(const Key('revision3-voice-folder-review')));
      await tester.pumpAndSettle();

      expect(capturedRequest?.folderPath, absoluteFolder);
      expect(capturedRequest?.locale, 'de');
      expect(
        find.byKey(const Key('revision3-voice-folder-wide-review')),
        findsOneWidget,
      );
      expect(
        find.text(
          '1 new ready · 1 already present · 0 blocked · 2 Ogg total · 1 other entries ignored',
        ),
        findsOneWidget,
      );
      expect(find.textContaining(projectId), findsNothing);
      expect(find.textContaining('deadbeef'), findsNothing);
      expect(find.textContaining('LOC_ASGHAN_SECRET'), findsNothing);
      expect(find.textContaining(r'C:\Private'), findsNothing);

      expect(
        find.byKey(const Key('revision3-voice-folder-exclusion-ack')),
        findsNothing,
      );
      expect(_applyButton(tester).onPressed, isNotNull);

      await tester.tap(find.byKey(const Key('revision3-voice-folder-apply')));
      await tester.pump();
      expect(applyCalls, 1);
      expect(capturedPlan, same(plan));
      expect(
        find.byKey(const Key('revision3-voice-folder-applying')),
        findsOneWidget,
      );
      expect(
        tester
            .widget<IconButton>(
              find.byKey(const Key('revision3-voice-folder-close')),
            )
            .onPressed,
        isNull,
      );

      await tester.tapAt(const Offset(2, 2));
      await tester.sendKeyEvent(LogicalKeyboardKey.escape);
      await tester.pump();
      expect(
        find.byKey(const Key('revision3-voice-folder-applying')),
        findsOneWidget,
      );

      applyCompletion.complete(_publication(plan));
      await tester.pumpAndSettle();
      expect(applyCalls, 1);
      expect(
        find.byKey(const Key('revision3-voice-folder-success')),
        findsOneWidget,
      );
      expect(find.textContaining('1 recording imported'), findsOneWidget);
    },
  );

  testWidgets('compact German review uses list/detail navigation and copy', (
    tester,
  ) async {
    await _useSurface(tester, const Size(700, 900));
    final plan = _plan(rows: [_readyRow(0)]);
    final service = Revision3VoiceFolderAuthoringService(
      planFolder: (_) async => plan,
      applyPlan: _unexpectedApply,
    );
    await _openRoute(
      tester,
      service: service,
      copy: const Revision3VoiceFolderImportDialogCopy.german(),
      picker: () async => r'C:\Voice\Aufnahmen',
    );

    expect(find.text('Voice-Aufnahmen importieren'), findsOneWidget);
    await _chooseAndReview(tester, settle: true);

    expect(
      find.byKey(const Key('revision3-voice-folder-compact-list')),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-voice-folder-wide-review')),
      findsNothing,
    );
    await tester.drag(
      find.byKey(const Key('revision3-voice-folder-row-browser')),
      const Offset(0, -300),
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.byKey(const ValueKey('revision3-voice-folder-row-0')),
    );
    await tester.pump();

    final details = find.byKey(
      const Key('revision3-voice-folder-compact-details'),
    );
    expect(details, findsOneWidget);
    await tester.drag(
      find.byKey(const Key('revision3-voice-folder-row-details')),
      const Offset(0, -320),
    );
    await tester.pumpAndSettle();
    expect(
      find.descendant(
        of: details,
        matching: find.textContaining('Aufgenommen'),
      ),
      findsWidgets,
    );
    expect(
      find.descendant(of: details, matching: find.textContaining('Recorded')),
      findsNothing,
    );
    await tester.tap(
      find.byKey(const Key('revision3-voice-folder-details-back')),
    );
    await tester.pump();
    expect(
      find.byKey(const Key('revision3-voice-folder-compact-list')),
      findsOneWidget,
    );
  });

  testWidgets('a blocked Ogg prevents partial apply and has no override', (
    tester,
  ) async {
    await _useSurface(tester, const Size(1300, 1000));
    final plan = _plan(rows: [_readyRow(0), _unmatchedRow(1)]);
    var applyCalls = 0;
    final service = Revision3VoiceFolderAuthoringService(
      planFolder: (_) async => plan,
      applyPlan: ({required plan}) async {
        applyCalls++;
        return _publication(plan);
      },
    );
    await _openRoute(
      tester,
      service: service,
      picker: () async => r'C:\Voice\Blocked Batch',
    );
    await _chooseAndReview(tester, settle: true);

    expect(
      find.byKey(const Key('revision3-voice-folder-blocked')),
      findsOneWidget,
    );
    expect(
      find.textContaining('There is no partial-import option'),
      findsWidgets,
    );
    expect(_applyButton(tester).onPressed, isNull);
    expect(
      find.byKey(const Key('revision3-voice-folder-exclusion-ack')),
      findsNothing,
    );
    expect(applyCalls, 0);
  });

  testWidgets('late plan completion is ignored after a project rebind', (
    tester,
  ) async {
    await _useSurface(tester, const Size(1300, 1000));
    final planCompletion = Completer<Revision3VoiceFolderImportPlan>();
    final harnessKey = GlobalKey<_DialogHarnessState>();
    final service = Revision3VoiceFolderAuthoringService(
      planFolder: (_) => planCompletion.future,
      applyPlan: _unexpectedApply,
    );
    await _pumpHarness(tester, key: harnessKey, service: service);
    await _chooseAndReview(tester);

    expect(
      find.byKey(const Key('revision3-voice-folder-planning')),
      findsOneWidget,
    );
    harnessKey.currentState!.rebind(
      projectId: 'another-project',
      revision: 1,
      head: 'another-head',
      checkpoint: 'another-checkpoint',
    );
    await tester.pump();
    expect(
      find.text(const Revision3VoiceFolderImportDialogCopy.english().stale),
      findsOneWidget,
    );

    planCompletion.complete(_plan());
    await tester.pump();
    expect(
      find.byKey(const Key('revision3-voice-folder-planning')),
      findsNothing,
    );
    expect(find.text('asghan_ready.ogg'), findsNothing);
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'exact revision rebind is accepted after one atomic publication',
    (tester) async {
      await _useSurface(tester, const Size(1300, 1000));
      final plan = _plan(rows: [_readyRow(0)]);
      final applyCompletion =
          Completer<Revision3VoiceFolderImportPublication>();
      final harnessKey = GlobalKey<_DialogHarnessState>();
      var applyCalls = 0;
      final service = Revision3VoiceFolderAuthoringService(
        planFolder: (_) async => plan,
        applyPlan: ({required plan}) {
          applyCalls++;
          return applyCompletion.future;
        },
      );
      await _pumpHarness(tester, key: harnessKey, service: service);
      await _chooseAndReview(tester, settle: true);
      await tester.tap(find.byKey(const Key('revision3-voice-folder-apply')));
      await tester.pump();

      harnessKey.currentState!.rebind(
        projectId: plan.projectId,
        revision: plan.projectRevision + 1,
        head: 'head-8',
        checkpoint: 'checkpoint-8',
      );
      await tester.pump();
      expect(
        find.byKey(const Key('revision3-voice-folder-applying')),
        findsOneWidget,
      );

      applyCompletion.complete(_publication(plan));
      await tester.pumpAndSettle();
      expect(applyCalls, 1);
      expect(
        find.byKey(const Key('revision3-voice-folder-success')),
        findsOneWidget,
      );
    },
  );

  testWidgets('late apply completion cannot publish into another project', (
    tester,
  ) async {
    await _useSurface(tester, const Size(1300, 1000));
    final plan = _plan(rows: [_readyRow(0)]);
    final applyCompletion = Completer<Revision3VoiceFolderImportPublication>();
    final harnessKey = GlobalKey<_DialogHarnessState>();
    final service = Revision3VoiceFolderAuthoringService(
      planFolder: (_) async => plan,
      applyPlan: ({required plan}) => applyCompletion.future,
    );
    await _pumpHarness(tester, key: harnessKey, service: service);
    await _chooseAndReview(tester, settle: true);
    await tester.tap(find.byKey(const Key('revision3-voice-folder-apply')));
    await tester.pump();

    harnessKey.currentState!.rebind(
      projectId: 'other-project',
      revision: 2,
      head: 'other-head',
      checkpoint: 'other-checkpoint',
    );
    await tester.pump();
    expect(
      find.text(const Revision3VoiceFolderImportDialogCopy.english().stale),
      findsOneWidget,
    );

    applyCompletion.complete(_publication(plan));
    await tester.pump();
    expect(
      find.byKey(const Key('revision3-voice-folder-success')),
      findsNothing,
    );
    expect(find.byKey(const Key('revision3-voice-folder-apply')), findsNothing);
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'raw failures stay hidden and uncertain publication is terminal',
    (tester) async {
      await _useSurface(tester, const Size(1300, 1000));
      final rawFailureService = Revision3VoiceFolderAuthoringService(
        planFolder: (_) async => throw StateError('RAW_SECRET_NATIVE_ERROR'),
        applyPlan: _unexpectedApply,
      );
      await _openRoute(
        tester,
        service: rawFailureService,
        picker: () async => r'C:\Voice\Batch',
      );
      await _chooseAndReview(tester, settle: true);

      expect(
        find.text(
          const Revision3VoiceFolderImportDialogCopy.english().planFailed,
        ),
        findsOneWidget,
      );
      expect(find.textContaining('RAW_SECRET'), findsNothing);
      await tester.tap(find.byKey(const Key('revision3-voice-folder-close')));
      await tester.pumpAndSettle();

      final uncertainService = Revision3VoiceFolderAuthoringService(
        planFolder: (_) async =>
            throw const Revision3VoiceFolderPublicationUncertainException(),
        applyPlan: _unexpectedApply,
      );
      await _openRoute(
        tester,
        service: uncertainService,
        picker: () async => r'C:\Voice\Batch',
      );
      await _chooseAndReview(tester, settle: true);

      expect(
        find.text(
          const Revision3VoiceFolderImportDialogCopy.english().uncertain,
        ),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-voice-folder-terminal')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-voice-folder-review')),
        findsNothing,
      );
    },
  );
}

Future<void> _useSurface(WidgetTester tester, Size size) async {
  await tester.binding.setSurfaceSize(size);
  addTearDown(() => tester.binding.setSurfaceSize(null));
}

FilledButton _applyButton(WidgetTester tester) => tester.widget<FilledButton>(
  find.byKey(const Key('revision3-voice-folder-apply')),
);

Future<void> _openRoute(
  WidgetTester tester, {
  required Revision3VoiceFolderAuthoringService service,
  Revision3VoiceFolderImportDialogCopy copy =
      const Revision3VoiceFolderImportDialogCopy.english(),
  Revision3VoiceFolderDirectoryPicker? picker,
  String projectId = 'project',
  int projectRevision = 7,
  String projectHead = 'head-7',
  String checkpointToken = 'checkpoint-7',
}) async {
  await tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: Builder(
          builder: (context) => FilledButton(
            key: const Key('open-folder-import'),
            onPressed: () => showRevision3VoiceFolderImportDialog(
              context: context,
              projectId: projectId,
              projectRevision: projectRevision,
              projectHead: projectHead,
              checkpointToken: checkpointToken,
              service: service,
              copy: copy,
              pickFolder: picker,
              initialLocale: 'de',
            ),
            child: const Text('Open'),
          ),
        ),
      ),
    ),
  );
  await tester.tap(find.byKey(const Key('open-folder-import')));
  await tester.pumpAndSettle();
}

Future<void> _pumpHarness(
  WidgetTester tester, {
  required GlobalKey<_DialogHarnessState> key,
  required Revision3VoiceFolderAuthoringService service,
}) => tester.pumpWidget(
  MaterialApp(
    home: Scaffold(
      body: _DialogHarness(key: key, service: service),
    ),
  ),
);

Future<void> _chooseAndReview(
  WidgetTester tester, {
  bool settle = false,
}) async {
  await tester.ensureVisible(
    find.byKey(const Key('revision3-voice-folder-pick')),
  );
  await tester.tap(find.byKey(const Key('revision3-voice-folder-pick')));
  await tester.pumpAndSettle();
  await tester.tap(find.byKey(const Key('revision3-voice-folder-review')));
  if (settle) {
    await tester.pumpAndSettle();
  } else {
    await tester.pump();
  }
}

class _DialogHarness extends StatefulWidget {
  const _DialogHarness({required this.service, super.key});

  final Revision3VoiceFolderAuthoringService service;

  @override
  State<_DialogHarness> createState() => _DialogHarnessState();
}

class _DialogHarnessState extends State<_DialogHarness> {
  String _projectId = 'project';
  int _revision = 7;
  String _head = 'head-7';
  String _checkpoint = 'checkpoint-7';

  void rebind({
    required String projectId,
    required int revision,
    required String head,
    required String checkpoint,
  }) => setState(() {
    _projectId = projectId;
    _revision = revision;
    _head = head;
    _checkpoint = checkpoint;
  });

  @override
  Widget build(BuildContext context) => Revision3VoiceFolderImportDialog(
    projectId: _projectId,
    projectRevision: _revision,
    projectHead: _head,
    checkpointToken: _checkpoint,
    service: widget.service,
    copy: const Revision3VoiceFolderImportDialogCopy.english(),
    pickFolder: () async => r'C:\Voice\Batch',
    initialLocale: 'de',
  );
}

Revision3VoiceFolderImportPlan _plan({
  String projectId = 'project',
  int projectRevision = 7,
  String projectHead = 'head-7',
  String checkpointToken = 'checkpoint-7',
  String planToken = 'plan-token',
  List<Revision3VoiceFolderReviewRow>? rows,
  int ignoredEntryCount = 0,
}) {
  final reviewRows = rows ?? [_readyRow(0)];
  return Revision3VoiceFolderImportPlan(
    projectId: projectId,
    projectRevision: projectRevision,
    projectHead: projectHead,
    checkpointToken: checkpointToken,
    planToken: planToken,
    folderLabel: 'Voice Batch',
    locale: 'de',
    scannedEntryCount: reviewRows.length + ignoredEntryCount,
    ignoredEntryCount: ignoredEntryCount,
    rows: reviewRows,
  );
}

Revision3VoiceFolderReviewRow _readyRow(int ordinal) =>
    Revision3VoiceFolderReviewRow(
      ordinal: ordinal,
      rowToken: 'LOC_ASGHAN_SECRET_ROW_$ordinal',
      status: Revision3VoiceFolderRowStatus.ready,
      codec: Revision3VoiceFolderCodec.vorbis,
      byteLength: 1536,
      lineLabel: 'Asghan — Mine entrance question',
      speakerLabel: 'Asghan',
      takeDisplayName: 'Asghan folder take',
      beforeTakeCount: 1,
      afterTakeCount: 2,
      targetState: Revision3VoiceFolderTargetState.resolved,
    );

Revision3VoiceFolderReviewRow _unmatchedRow(int ordinal) =>
    Revision3VoiceFolderReviewRow(
      ordinal: ordinal,
      rowToken: 'unmatched-secret-$ordinal',
      status: Revision3VoiceFolderRowStatus.unmatched,
      codec: Revision3VoiceFolderCodec.opus,
      byteLength: 2048,
      lineLabel: null,
      speakerLabel: null,
      takeDisplayName: null,
      beforeTakeCount: null,
      afterTakeCount: null,
      targetState: null,
    );

Revision3VoiceFolderReviewRow _alreadyPresentRow(int ordinal) =>
    Revision3VoiceFolderReviewRow(
      ordinal: ordinal,
      rowToken: 'existing-secret-$ordinal',
      status: Revision3VoiceFolderRowStatus.alreadyPresent,
      codec: Revision3VoiceFolderCodec.vorbis,
      byteLength: 1536,
      lineLabel: 'Asghan — Mine entrance question',
      speakerLabel: 'Asghan',
      takeDisplayName: null,
      beforeTakeCount: 1,
      afterTakeCount: 1,
      targetState: Revision3VoiceFolderTargetState.resolved,
    );

Revision3VoiceFolderImportPublication _publication(
  Revision3VoiceFolderImportPlan plan,
) => Revision3VoiceFolderImportPublication(
  projectId: plan.projectId,
  projectRevision: plan.projectRevision + 1,
  projectHead: 'head-8',
  checkpointToken: 'checkpoint-8',
  planToken: plan.planToken,
  importedCount: plan.counts.ready,
);

Future<Revision3VoiceFolderImportPublication> _unexpectedApply({
  required Revision3VoiceFolderImportPlan plan,
}) => throw StateError('apply was not expected');
