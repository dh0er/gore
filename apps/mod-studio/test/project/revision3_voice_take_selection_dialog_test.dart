import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_voice_take_selection_authoring.dart';
import 'package:gore_mod/project/revision3_voice_take_selection_dialog.dart';

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
}

Future<void> _openDialog(
  WidgetTester tester, {
  required Revision3ContentIndex index,
  Future<Revision3ContentIndex> Function()? load,
  Revision3VoiceTakeSelectionTechnicalPublisher? publish,
}) async {
  await tester.binding.setSurfaceSize(const Size(1200, 1000));
  addTearDown(() => tester.binding.setSurfaceSize(null));
  final service = Revision3VoiceTakeSelectionAuthoringService(
    loadContentIndex: load ?? () async => index,
    publishTechnicalPlan: publish ?? _unexpectedPublish,
  );
  await tester.pumpWidget(
    MaterialApp(
      home: Builder(
        builder: (context) => FilledButton(
          key: const Key('open-dialog'),
          onPressed: () => showDialog<Revision3VoiceTakeSelectionPublication>(
            context: context,
            builder: (_) => Revision3VoiceTakeSelectionDialog(service: service),
          ),
          child: const Text('Open'),
        ),
      ),
    ),
  );
  await tester.tap(find.byKey(const Key('open-dialog')));
  await tester.pumpAndSettle();
}

Revision3ContentIndex _index({bool secondApproved = false}) {
  final json = revision3VoiceContentIndexJsonFixture(
    existingSlotCandidateCount: 2,
    existingSlotHasSelectedTake: true,
  );
  if (secondApproved) {
    final takes = (json['entities']! as List)
        .cast<Map<String, Object?>>()
        .where((entity) => entity['kind'] == 'voice_take')
        .toList(growable: false);
    final summary = (takes[1]['summary']! as Map).cast<String, Object?>();
    final data = (summary['data']! as Map).cast<String, Object?>();
    data['status'] = 'approved';
    summary['data'] = data;
    takes[1]['summary'] = summary;
  }
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

Future<Revision3VoiceTakeSelectionPublication> _unexpectedPublish({
  required String expectedProjectId,
  required int expectedProjectRevision,
  required Revision3VoiceTakeSelectionTechnicalPlan plan,
}) => throw StateError('publisher was not expected');

ButtonStyleButton _button(WidgetTester tester, String key) =>
    tester.widget<ButtonStyleButton>(find.byKey(Key(key)));
