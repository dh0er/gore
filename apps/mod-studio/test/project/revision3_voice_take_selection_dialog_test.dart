import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_voice_take_selection_authoring.dart';
import 'package:gore_mod/project/revision3_voice_take_selection_dialog.dart';
import 'package:gore_mod/project/revision3_voice_take_status_authoring.dart';

import '../support/revision3_voice_content_fixture.dart';

void main() {
  testWidgets(
    'dialog has no implicit line choice, marks current, disables no-op and nonapproved',
    (tester) async {
      await _openDialog(tester, index: _index());

      expect(find.byKey(const Key('voice-selection-save')), findsOneWidget);
      expect(_button(tester, 'voice-selection-save').onPressed, isNull);
      expect(find.textContaining(revision3VoiceContentLineId), findsNothing);
      expect(find.textContaining(revision3VoiceContentSlotId), findsNothing);
      expect(
        find.descendant(
          of: find.byKey(const Key('voice-selection-lines')),
          matching: find.textContaining('GRD_263_ASGHAN_OPEN_INFO_06_02'),
        ),
        findsNothing,
      );
      expect(find.textContaining(r'C:\'), findsNothing);

      await tester.tap(find.byKey(const Key('voice-selection-line-0')));
      await tester.pump();

      expect(find.textContaining('Current selection'), findsOneWidget);
      expect(_button(tester, 'voice-selection-save').onPressed, isNull);
      final recorded = tester.widget<RadioListTile<String>>(
        find.byKey(const Key('voice-selection-take-1')),
      );
      expect(recorded.enabled, isFalse);
      expect(find.textContaining('Approval required'), findsOneWidget);
    },
  );

  testWidgets(
    'clear warns, stays explicit, and publishes a project-only clear',
    (tester) async {
      Revision3VoiceTakeSelectionTechnicalPlan? received;
      await _openDialog(
        tester,
        index: _index(),
        publish:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) async {
              received = plan;
              return _publication(
                projectId: expectedProjectId,
                revision: expectedProjectRevision + 1,
                plan: plan,
              );
            },
      );
      await tester.tap(find.byKey(const Key('voice-selection-line-0')));
      await tester.pump();
      await tester.tap(find.byKey(const Key('voice-selection-clear')));
      await tester.pump();

      expect(
        find.byKey(const Key('voice-selection-clear-warning')),
        findsOneWidget,
      );
      expect(find.textContaining('takes stay in this project'), findsOneWidget);
      expect(_button(tester, 'voice-selection-save').onPressed, isNotNull);

      await tester.tap(find.byKey(const Key('voice-selection-save')));
      await tester.pumpAndSettle();

      expect(received, isNotNull);
      expect(received!.selectedTakeId, isNull);
      expect(received!.expectedSelectedTakeId, isNotNull);
      expect(
        find.byKey(const Key('revision3-voice-take-selection-dialog')),
        findsNothing,
      );
    },
  );

  testWidgets('only an Approved alternate can be selected and saved', (
    tester,
  ) async {
    final index = _index(secondApproved: true);
    Revision3VoiceTakeSelectionTechnicalPlan? received;
    await _openDialog(
      tester,
      index: index,
      publish:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            received = plan;
            return _publication(
              projectId: expectedProjectId,
              revision: expectedProjectRevision + 1,
              plan: plan,
            );
          },
    );
    await tester.tap(find.byKey(const Key('voice-selection-line-0')));
    await tester.pump();
    final alternate = tester.widget<RadioListTile<String>>(
      find.byKey(const Key('voice-selection-take-1')),
    );
    expect(alternate.enabled, isTrue);
    await tester.tap(find.byKey(const Key('voice-selection-take-1')));
    await tester.pump();
    expect(_button(tester, 'voice-selection-save').onPressed, isNotNull);
    await tester.tap(find.byKey(const Key('voice-selection-save')));
    await tester.pumpAndSettle();

    expect(received?.selectedTakeId, isNotNull);
    expect(received?.selectedTakeId, isNot(received?.expectedSelectedTakeId));
  });

  testWidgets('busy state locks controls and exposes no premature result', (
    tester,
  ) async {
    final completer = Completer<Revision3VoiceTakeSelectionPublication>();
    Revision3VoiceTakeSelectionTechnicalPlan? received;
    await _openDialog(
      tester,
      index: _index(),
      publish:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) {
            received = plan;
            return completer.future;
          },
    );
    await tester.tap(find.byKey(const Key('voice-selection-line-0')));
    await tester.pump();
    await tester.tap(find.byKey(const Key('voice-selection-clear')));
    await tester.pump();
    await tester.tap(find.byKey(const Key('voice-selection-save')));
    await tester.pump();

    expect(find.byType(CircularProgressIndicator), findsOneWidget);
    expect(_button(tester, 'voice-selection-cancel').onPressed, isNull);
    expect(
      tester
          .widget<TextField>(
            find.byKey(const Key('voice-selection-line-search')),
          )
          .enabled,
      isFalse,
    );

    completer.complete(
      _publication(
        projectId: revision3VoiceContentProjectId,
        revision: 8,
        plan: received!,
      ),
    );
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key('revision3-voice-take-selection-dialog')),
      findsNothing,
    );
  });

  testWidgets('fresh-index drift is shown as stale and keeps dialog open', (
    tester,
  ) async {
    var reads = 0;
    final initial = _index(secondApproved: true);
    final stale = revision3VoiceContentIndexFixture(
      revision: initial.projectRevision + 1,
      existingSlotCandidateCount: 2,
      existingSlotHasSelectedTake: true,
    );
    await _openDialog(
      tester,
      index: initial,
      load: () async => reads++ == 0 ? initial : stale,
    );
    await tester.tap(find.byKey(const Key('voice-selection-line-0')));
    await tester.pump();
    await tester.tap(find.byKey(const Key('voice-selection-take-1')));
    await tester.pump();
    await tester.tap(find.byKey(const Key('voice-selection-save')));
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('voice-selection-error')), findsOneWidget);
    expect(find.textContaining('project changed'), findsOneWidget);
    expect(
      find.byKey(const Key('revision3-voice-take-selection-dialog')),
      findsOneWidget,
    );
  });

  testWidgets('reopen load failure is friendly and retryable', (tester) async {
    await _openDialog(
      tester,
      index: _index(),
      load: () async => throw const Revision3ContentRequiresReopenException(),
    );
    await tester.pumpAndSettle();

    expect(find.textContaining('Reopen the managed project'), findsOneWidget);
    expect(find.byKey(const Key('voice-selection-retry')), findsOneWidget);
  });

  testWidgets('shows workflow status and locks the selected Approved take', (
    tester,
  ) async {
    await _openDialog(tester, index: _index());
    await tester.tap(find.byKey(const Key('voice-selection-line-0')));
    await tester.pump();

    expect(find.textContaining('author workflow label'), findsOneWidget);
    expect(find.textContaining('in-game readiness'), findsOneWidget);
    expect(find.textContaining('Approved • Current selection'), findsOneWidget);
    expect(find.textContaining('Recorded'), findsOneWidget);
    expect(
      tester
          .widget<PopupMenuButton<AuthoringRevision3VoiceTakeStatus>>(
            find.byKey(const Key('voice-status-change-0')),
          )
          .enabled,
      isFalse,
    );
    expect(find.textContaining('Clear the selection'), findsOneWidget);

    final alternate = find.byKey(const Key('voice-status-change-1'));
    expect(
      tester
          .widget<PopupMenuButton<AuthoringRevision3VoiceTakeStatus>>(alternate)
          .enabled,
      isTrue,
    );
    await tester.tap(alternate);
    await tester.pumpAndSettle();
    for (final status in AuthoringRevision3VoiceTakeStatus.values) {
      final item = find.byKey(Key('voice-status-option-1-${status.name}'));
      expect(item, findsOneWidget);
      expect(
        tester
            .widget<PopupMenuItem<AuthoringRevision3VoiceTakeStatus>>(item)
            .enabled,
        status == AuthoringRevision3VoiceTakeStatus.recorded ? isFalse : isTrue,
      );
    }
  });

  testWidgets('historical selected Recorded take can only become Approved', (
    tester,
  ) async {
    await _openDialog(tester, index: _index(selectedStatus: 'recorded'));
    await tester.tap(find.byKey(const Key('voice-selection-line-0')));
    await tester.pump();

    expect(
      find.textContaining(
        'Current selection must be Approved; change to Approved or clear it',
      ),
      findsOneWidget,
    );
    final control = find.byKey(const Key('voice-status-change-0'));
    expect(
      tester
          .widget<PopupMenuButton<AuthoringRevision3VoiceTakeStatus>>(control)
          .enabled,
      isTrue,
    );
    await tester.tap(control);
    await tester.pumpAndSettle();
    for (final status in AuthoringRevision3VoiceTakeStatus.values) {
      final item = find.byKey(Key('voice-status-option-0-${status.name}'));
      expect(
        tester
            .widget<PopupMenuItem<AuthoringRevision3VoiceTakeStatus>>(item)
            .enabled,
        status == AuthoringRevision3VoiceTakeStatus.approved ? isTrue : isFalse,
      );
    }
  });

  testWidgets('Approved status reloads exactly and is immediately selectable', (
    tester,
  ) async {
    var current = _index();
    var statusCalls = 0;
    Revision3VoiceTakeStatusTechnicalPlan? statusPlan;
    Revision3VoiceTakeSelectionTechnicalPlan? selectionPlan;
    await _openDialog(
      tester,
      index: current,
      load: () async => current,
      publishStatus:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            statusCalls++;
            statusPlan = plan;
            expect(expectedProjectRevision, 7);
            current = _index(
              revision: 8,
              secondApproved: true,
              secondTakeRevision: 1,
            );
            return _statusPublication(
              projectId: expectedProjectId,
              revision: 8,
              plan: plan,
            );
          },
      publish:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            selectionPlan = plan;
            expect(expectedProjectRevision, 8);
            return _publication(
              projectId: expectedProjectId,
              revision: 9,
              plan: plan,
            );
          },
    );
    await tester.tap(find.byKey(const Key('voice-selection-line-0')));
    await tester.pump();
    await tester.tap(find.byKey(const Key('voice-status-change-1')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('voice-status-option-1-approved')));
    await tester.pumpAndSettle();

    expect(statusCalls, 1);
    expect(
      statusPlan?.desiredStatus,
      AuthoringRevision3VoiceTakeStatus.approved,
    );
    expect(find.textContaining('can now be selected'), findsOneWidget);
    expect(find.widgetWithText(TextButton, 'Close'), findsOneWidget);
    expect(
      tester
          .widget<RadioListTile<String>>(
            find.byKey(const Key('voice-selection-take-1')),
          )
          .enabled,
      isTrue,
    );

    await tester.tap(find.byKey(const Key('voice-selection-take-1')));
    await tester.pump();
    expect(
      find.byKey(const Key('voice-status-selection-pending')),
      findsOneWidget,
    );
    expect(
      tester
          .widget<PopupMenuButton<AuthoringRevision3VoiceTakeStatus>>(
            find.byKey(const Key('voice-status-change-1')),
          )
          .enabled,
      isFalse,
    );
    await tester.tap(find.byKey(const Key('voice-selection-save')));
    await tester.pumpAndSettle();

    expect(selectionPlan?.selectedTakeId, statusPlan?.takeId);
    expect(
      find.byKey(const Key('revision3-voice-take-selection-dialog')),
      findsNothing,
    );
  });

  testWidgets('status publish busy state blocks every second action', (
    tester,
  ) async {
    var current = _index();
    var publishCalls = 0;
    Revision3VoiceTakeStatusTechnicalPlan? received;
    final completer = Completer<Revision3VoiceTakeStatusPublication>();
    await _openDialog(
      tester,
      index: current,
      load: () async => current,
      publishStatus:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) {
            publishCalls++;
            received = plan;
            return completer.future;
          },
    );
    await tester.tap(find.byKey(const Key('voice-selection-line-0')));
    await tester.pump();
    await tester.tap(find.byKey(const Key('voice-status-change-1')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('voice-status-option-1-approved')));
    await tester.pump();

    expect(publishCalls, 1);
    expect(_button(tester, 'voice-selection-cancel').onPressed, isNull);
    expect(
      tester
          .widget<TextField>(
            find.byKey(const Key('voice-selection-line-search')),
          )
          .enabled,
      isFalse,
    );
    expect(
      tester
          .widget<RadioListTile<String>>(
            find.byKey(const Key('voice-selection-clear')),
          )
          .enabled,
      isFalse,
    );

    current = _index(revision: 8, secondApproved: true, secondTakeRevision: 1);
    completer.complete(
      _statusPublication(
        projectId: revision3VoiceContentProjectId,
        revision: 8,
        plan: received!,
      ),
    );
    await tester.pumpAndSettle();
    expect(publishCalls, 1);
  });

  testWidgets('reload recovery never repeats an already saved status', (
    tester,
  ) async {
    final initial = _index();
    final refreshed = _index(
      revision: 8,
      secondApproved: true,
      secondTakeRevision: 1,
    );
    var reads = 0;
    var publishCalls = 0;
    await _openDialog(
      tester,
      index: initial,
      load: () async {
        reads++;
        if (reads == 3) throw StateError('temporary reload failure');
        return reads >= 4 ? refreshed : initial;
      },
      publishStatus:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            publishCalls++;
            return _statusPublication(
              projectId: expectedProjectId,
              revision: 8,
              plan: plan,
            );
          },
    );
    await tester.tap(find.byKey(const Key('voice-selection-line-0')));
    await tester.pump();
    await tester.tap(find.byKey(const Key('voice-status-change-1')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('voice-status-option-1-approved')));
    await tester.pumpAndSettle();

    expect(publishCalls, 1);
    expect(find.textContaining('status was saved'), findsOneWidget);
    expect(find.byKey(const Key('voice-status-reload')), findsOneWidget);
    expect(_button(tester, 'voice-selection-save').onPressed, isNull);

    await tester.tap(find.byKey(const Key('voice-status-reload')));
    await tester.pumpAndSettle();

    expect(publishCalls, 1);
    expect(find.byKey(const Key('voice-status-reload')), findsNothing);
    expect(find.textContaining('Saved status confirmed'), findsOneWidget);
    expect(
      tester
          .widget<RadioListTile<String>>(
            find.byKey(const Key('voice-selection-take-1')),
          )
          .enabled,
      isTrue,
    );
  });

  testWidgets('reload requiring reopen leaves only Close enabled', (
    tester,
  ) async {
    final initial = _index();
    var reads = 0;
    var publishCalls = 0;
    await _openDialog(
      tester,
      index: initial,
      load: () async {
        reads++;
        if (reads == 3) throw StateError('temporary reload failure');
        if (reads >= 4) {
          throw const Revision3ContentRequiresReopenException();
        }
        return initial;
      },
      publishStatus:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            publishCalls++;
            return _statusPublication(
              projectId: expectedProjectId,
              revision: 8,
              plan: plan,
            );
          },
    );
    await tester.tap(find.byKey(const Key('voice-selection-line-0')));
    await tester.pump();
    await tester.tap(find.byKey(const Key('voice-status-change-1')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('voice-status-option-1-approved')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('voice-status-reload')));
    await tester.pumpAndSettle();

    expect(publishCalls, 1);
    expect(find.byKey(const Key('voice-status-reload')), findsNothing);
    expect(find.textContaining('reopen the managed project'), findsOneWidget);
    expect(_button(tester, 'voice-selection-save').onPressed, isNull);
    expect(_button(tester, 'voice-selection-cancel').onPressed, isNotNull);
    expect(find.widgetWithText(TextButton, 'Close'), findsOneWidget);
  });
}

Future<void> _openDialog(
  WidgetTester tester, {
  required Revision3ContentIndex index,
  Future<Revision3ContentIndex> Function()? load,
  Revision3VoiceTakeSelectionTechnicalPublisher? publish,
  Revision3VoiceTakeStatusTechnicalPublisher? publishStatus,
}) async {
  await tester.binding.setSurfaceSize(const Size(1200, 1000));
  addTearDown(() => tester.binding.setSurfaceSize(null));
  final service = Revision3VoiceTakeSelectionAuthoringService(
    loadContentIndex: load ?? () async => index,
    publishTechnicalPlan: publish ?? _unexpectedPublish,
  );
  final statusService = Revision3VoiceTakeStatusAuthoringService(
    loadContentIndex: load ?? () async => index,
    publishTechnicalPlan: publishStatus ?? _unexpectedStatusPublish,
  );
  await tester.pumpWidget(
    MaterialApp(
      home: Builder(
        builder: (context) => FilledButton(
          key: const Key('open-dialog'),
          onPressed: () => showDialog<Revision3VoiceTakeSelectionPublication>(
            context: context,
            builder: (_) => Revision3VoiceTakeSelectionDialog(
              service: service,
              statusService: statusService,
            ),
          ),
          child: const Text('Open'),
        ),
      ),
    ),
  );
  await tester.tap(find.byKey(const Key('open-dialog')));
  await tester.pumpAndSettle();
}

Revision3ContentIndex _index({
  int revision = 7,
  bool secondApproved = false,
  int secondTakeRevision = 0,
  String selectedStatus = 'approved',
}) {
  final json = revision3VoiceContentIndexJsonFixture(
    revision: revision,
    existingSlotCandidateCount: 2,
    existingSlotHasSelectedTake: true,
  );
  final takes = (json['entities']! as List)
      .cast<Map<String, Object?>>()
      .where((entity) => entity['kind'] == 'voice_take')
      .toList(growable: false);
  final selectedSummary = (takes[0]['summary']! as Map).cast<String, Object?>();
  final selectedData = (selectedSummary['data']! as Map)
      .cast<String, Object?>();
  selectedData['status'] = selectedStatus;
  selectedSummary['data'] = selectedData;
  takes[0]['summary'] = selectedSummary;
  if (secondApproved) {
    final summary = (takes[1]['summary']! as Map).cast<String, Object?>();
    final data = (summary['data']! as Map).cast<String, Object?>();
    data['status'] = 'approved';
    summary['data'] = data;
    takes[1]['summary'] = summary;
  }
  takes[1]['revision'] = secondTakeRevision;
  return Revision3ContentIndex.fromJsonObject(json);
}

Revision3VoiceTakeSelectionPublication _publication({
  required String projectId,
  required int revision,
  required Revision3VoiceTakeSelectionTechnicalPlan plan,
}) => Revision3VoiceTakeSelectionPublication(
  projectId: projectId,
  projectRevision: revision,
  lineId: plan.lineId,
  slotId: plan.slotId,
  slotRevision: plan.expectedSlotRevision + 1,
  locale: plan.locale,
  locId: plan.locId,
  previousSelectedTakeId: plan.expectedSelectedTakeId,
  selectedTakeId: plan.selectedTakeId,
);

Revision3VoiceTakeStatusPublication _statusPublication({
  required String projectId,
  required int revision,
  required Revision3VoiceTakeStatusTechnicalPlan plan,
}) => Revision3VoiceTakeStatusPublication(
  projectId: projectId,
  projectRevision: revision,
  lineId: plan.lineId,
  localizationId: plan.localizationId,
  slotId: plan.slotId,
  slotRevision: plan.expectedSlotRevision,
  locale: plan.locale,
  locId: plan.locId,
  takeId: plan.takeId,
  takeRevision: plan.expectedTakeRevision + 1,
  previousStatus: plan.expectedStatus,
  status: plan.desiredStatus,
);

Future<Revision3VoiceTakeSelectionPublication> _unexpectedPublish({
  required String expectedProjectId,
  required int expectedProjectRevision,
  required Revision3VoiceTakeSelectionTechnicalPlan plan,
}) => throw StateError('publisher was not expected');

Future<Revision3VoiceTakeStatusPublication> _unexpectedStatusPublish({
  required String expectedProjectId,
  required int expectedProjectRevision,
  required Revision3VoiceTakeStatusTechnicalPlan plan,
}) => throw StateError('status publisher was not expected');

ButtonStyleButton _button(WidgetTester tester, String key) =>
    tester.widget<ButtonStyleButton>(find.byKey(Key(key)));
