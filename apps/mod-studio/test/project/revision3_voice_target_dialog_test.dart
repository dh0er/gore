import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_voice_authoring.dart';
import 'package:gore_mod/project/revision3_voice_target_dialog.dart';

import '../support/revision3_voice_content_fixture.dart';

void main() {
  testWidgets(
    'offers only existing safe slots and returns the exact publication',
    (tester) async {
      var loads = 0;
      var publishes = 0;
      Revision3VoiceTargetTechnicalPlan? publishedPlan;
      Revision3VoiceTargetPublication? dialogResult;
      final service = Revision3VoiceTargetAuthoringService(
        loadContentIndex: () async {
          loads += 1;
          return revision3VoiceContentIndexFixture(duplicateLine: true);
        },
        publishTechnicalPlan:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) async {
              publishes += 1;
              publishedPlan = plan;
              return _publication(
                plan,
                projectId: expectedProjectId,
                revision: expectedProjectRevision + 1,
              );
            },
      );

      await _openDialog(
        tester,
        service,
        onResult: (value) => dialogResult = value,
      );

      expect(
        find.textContaining('does not change the archive'),
        findsOneWidget,
      );
      expect(find.textContaining('Does not deploy'), findsOneWidget);
      await _chooseAsghanLine(tester);

      final results = find.byKey(
        const Key('revision3-voice-target-line-results'),
      );
      expect(
        find.descendant(of: results, matching: find.byType(ListTile)),
        findsNothing,
      );
      expect(find.text(revision3VoiceContentLineId), findsNothing);
      expect(find.text(revision3VoiceContentSlotId), findsNothing);
      expect(find.text(revision3VoiceContentDuplicateLineId), findsNothing);
      expect(find.text('Current target: unresolved'), findsOneWidget);

      await tester.tap(find.byType(DropdownButtonFormField<String>));
      await tester.pumpAndSettle();
      expect(
        tester
            .widgetList<DropdownMenuItem<String>>(
              find.byType(DropdownMenuItem<String>),
            )
            .map((item) => item.value)
            .toSet(),
        <String>{'de'},
      );
      expect(find.text('en'), findsNothing);
      await tester.tap(find.text('de').last);
      await tester.pumpAndSettle();

      await tester.tap(find.byKey(const Key('revision3-voice-target-submit')));
      await tester.pumpAndSettle();

      expect(loads, 2);
      expect(publishes, 1);
      expect(publishedPlan?.lineId, revision3VoiceContentLineId);
      expect(publishedPlan?.slotId, revision3VoiceContentSlotId);
      expect(publishedPlan?.locale, 'de');
      expect(publishedPlan?.locId, 'GRD_263_ASGHAN_OPEN_INFO_06_02');
      expect(dialogResult?.projectRevision, 8);
      expect(dialogResult?.matchCount, 1);
      expect(
        dialogResult?.resolution,
        AuthoringRevision3VoiceTargetResolutionState.resolved,
      );
      expect(
        find.byKey(const Key('revision3-voice-target-dialog')),
        findsNothing,
      );
    },
  );

  testWidgets('context handoff preselects the exact line and Voice language', (
    tester,
  ) async {
    final service = Revision3VoiceTargetAuthoringService(
      loadContentIndex: () async => revision3VoiceContentIndexFixture(),
      publishTechnicalPlan: _unexpectedPublish,
    );
    await _openDialog(
      tester,
      service,
      initialLineId: revision3VoiceContentLineId,
      initialLocale: 'de',
    );

    expect(find.text('Current target: unresolved'), findsOneWidget);
    expect(
      tester
          .widget<DropdownButtonFormField<String>>(
            find.byType(DropdownButtonFormField<String>),
          )
          .initialValue,
      'de',
    );
    expect(find.text(revision3VoiceContentLineId), findsNothing);
  });

  testWidgets(
    'fixed context hides global navigation and resolves only its exact slot',
    (tester) async {
      Revision3VoiceTargetTechnicalPlan? received;
      final service = Revision3VoiceTargetAuthoringService(
        loadContentIndex: () async => revision3VoiceContentIndexFixture(),
        publishTechnicalPlan:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) async {
              received = plan;
              return _publication(
                plan,
                projectId: expectedProjectId,
                revision: expectedProjectRevision + 1,
              );
            },
      );

      await _openDialog(
        tester,
        service,
        initialLineId: revision3VoiceContentLineId,
        initialLocale: 'de',
        fixedContext: true,
      );

      expect(
        find.byKey(const Key('revision3-voice-target-fixed-context')),
        findsOneWidget,
      );
      expect(find.textContaining('Mine entrance question'), findsOneWidget);
      expect(find.text('Voice language: de'), findsOneWidget);
      expect(
        find.byKey(const Key('revision3-voice-target-line-search')),
        findsNothing,
      );
      expect(find.byType(DropdownButtonFormField<String>), findsNothing);
      expect(find.text(revision3VoiceContentLineId), findsNothing);

      await tester.tap(find.byKey(const Key('revision3-voice-target-submit')));
      await tester.pumpAndSettle();

      expect(received?.lineId, revision3VoiceContentLineId);
      expect(received?.locale, 'de');
    },
  );

  testWidgets('invalid fixed context fails closed without resolving', (
    tester,
  ) async {
    var publishes = 0;
    final service = Revision3VoiceTargetAuthoringService(
      loadContentIndex: () async => revision3VoiceContentIndexFixture(),
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            publishes++;
            throw StateError('must not publish');
          },
    );

    await _openDialog(
      tester,
      service,
      initialLineId: revision3VoiceContentLineId,
      initialLocale: 'en',
      fixedContext: true,
    );

    expect(find.textContaining('no longer matches'), findsOneWidget);
    expect(
      find.byKey(const Key('revision3-voice-target-line-search')),
      findsNothing,
    );
    expect(find.byType(DropdownButtonFormField<String>), findsNothing);
    expect(_submitButton(tester).onPressed, isNull);
    expect(publishes, 0);
  });

  testWidgets('offers an intact target even when its take capacity is full', (
    tester,
  ) async {
    var publishes = 0;
    final service = Revision3VoiceTargetAuthoringService(
      loadContentIndex: () async =>
          revision3VoiceContentIndexFixture(existingSlotCandidateCount: 1024),
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            publishes += 1;
            return _publication(
              plan,
              projectId: expectedProjectId,
              revision: expectedProjectRevision + 1,
            );
          },
    );

    await _openDialog(tester, service);
    await _chooseAsghanLine(tester);

    expect(find.text('Current target: unresolved'), findsOneWidget);
    expect(_submitButton(tester).onPressed, isNotNull);
    await tester.tap(find.byKey(const Key('revision3-voice-target-submit')));
    await tester.pumpAndSettle();
    expect(publishes, 1);
  });

  testWidgets('shows an honest empty state when no existing slot is safe', (
    tester,
  ) async {
    final service = Revision3VoiceTargetAuthoringService(
      loadContentIndex: () async =>
          revision3VoiceContentIndexFixture(existingDeSlot: false),
      publishTechnicalPlan: _unexpectedPublish,
    );

    await _openDialog(tester, service);

    expect(
      find.byKey(const Key('revision3-voice-target-empty')),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-voice-target-line-search')),
      findsNothing,
    );
    expect(_submitButton(tester).onPressed, isNull);
  });

  testWidgets('blocks publication when the catalog checkpoint became stale', (
    tester,
  ) async {
    var loads = 0;
    var publishes = 0;
    final service = Revision3VoiceTargetAuthoringService(
      loadContentIndex: () async {
        loads += 1;
        return revision3VoiceContentIndexFixture(revision: loads == 1 ? 7 : 8);
      },
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            publishes += 1;
            return _publication(
              plan,
              projectId: expectedProjectId,
              revision: expectedProjectRevision + 1,
            );
          },
    );

    await _openDialog(tester, service);
    await _chooseAsghanLine(tester);
    await tester.tap(find.byKey(const Key('revision3-voice-target-submit')));
    await tester.pumpAndSettle();

    expect(loads, 2);
    expect(publishes, 0);
    expect(
      find.textContaining('managed project changed while this window was open'),
      findsOneWidget,
    );
    expect(_submitButton(tester).onPressed, isNull);
    expect(find.text('Close'), findsOneWidget);
  });

  testWidgets('requires reopening after a mismatched publication receipt', (
    tester,
  ) async {
    final service = Revision3VoiceTargetAuthoringService(
      loadContentIndex: () async => revision3VoiceContentIndexFixture(),
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async => _publication(
            plan,
            projectId: expectedProjectId,
            revision: expectedProjectRevision + 2,
          ),
    );

    await _openDialog(tester, service);
    await _chooseAsghanLine(tester);
    await tester.tap(find.byKey(const Key('revision3-voice-target-submit')));
    await tester.pumpAndSettle();

    expect(
      find.textContaining('can no longer be verified as current'),
      findsOneWidget,
    );
    expect(_submitButton(tester).onPressed, isNull);
    expect(find.text('Close'), findsOneWidget);
  });

  testWidgets('fresh catalog lease poison is terminal and cannot retry', (
    tester,
  ) async {
    var loads = 0;
    var publishes = 0;
    final service = Revision3VoiceTargetAuthoringService(
      loadContentIndex: () async {
        loads += 1;
        if (loads == 2) {
          throw const Revision3ContentRequiresReopenException();
        }
        return revision3VoiceContentIndexFixture();
      },
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            publishes += 1;
            return _publication(
              plan,
              projectId: expectedProjectId,
              revision: expectedProjectRevision + 1,
            );
          },
    );

    await _openDialog(tester, service);
    await _chooseAsghanLine(tester);
    await tester.tap(find.byKey(const Key('revision3-voice-target-submit')));
    await tester.pumpAndSettle();

    expect(loads, 2);
    expect(publishes, 0);
    expect(
      find.textContaining('can no longer be verified as current'),
      findsOneWidget,
    );
    expect(_submitButton(tester).onPressed, isNull);
    expect(find.byKey(const Key('revision3-voice-target-retry')), findsNothing);
    expect(find.text('Close'), findsOneWidget);
  });

  testWidgets('maps native failures without exposing technical details', (
    tester,
  ) async {
    final service = Revision3VoiceTargetAuthoringService(
      loadContentIndex: () async => revision3VoiceContentIndexFixture(
        existingSlotTargetResolution: 'ambiguous',
      ),
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async => throw const ModFfiException(
            command: 'authoring_revision3_voice_target',
            code: 'AUTHORING_REVISION3_VOICE_TARGET_ARCHIVE_UNAVAILABLE',
            message: r'C:\private\install\Voice.pak could not be opened',
          ),
    );

    await _openDialog(tester, service);
    await _chooseAsghanLine(tester);
    expect(find.text('Current target: ambiguous'), findsOneWidget);
    await tester.tap(find.byKey(const Key('revision3-voice-target-submit')));
    await tester.pumpAndSettle();

    expect(
      find.textContaining('installed Voice archive for this language'),
      findsOneWidget,
    );
    expect(find.textContaining(r'C:\private'), findsNothing);
    expect(_submitButton(tester).onPressed, isNotNull);
  });

  testWidgets('explains executable generation failures specifically', (
    tester,
  ) async {
    for (final entry in <(String, String)>[
      (
        'AUTHORING_REVISION3_VOICE_TARGET_EXECUTABLE_UNAVAILABLE',
        'game executable could not be read',
      ),
      (
        'AUTHORING_REVISION3_VOICE_TARGET_EXECUTABLE_MISMATCH',
        'no longer matches this project generation',
      ),
    ]) {
      final service = Revision3VoiceTargetAuthoringService(
        loadContentIndex: () async => revision3VoiceContentIndexFixture(),
        publishTechnicalPlan:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) async => throw ModFfiException(
              command: 'authoring_revision3_voice_target',
              code: entry.$1,
              message: 'private native detail',
            ),
      );
      await _openDialog(tester, service);
      await _chooseAsghanLine(tester);
      await tester.tap(find.byKey(const Key('revision3-voice-target-submit')));
      await tester.pumpAndSettle();
      expect(find.textContaining(entry.$2), findsOneWidget);
      expect(find.textContaining('private native detail'), findsNothing);
      await tester.tap(find.byKey(const Key('revision3-voice-target-cancel')));
      await tester.pumpAndSettle();
    }
  });

  testWidgets('German fixed context resolves with localized safety copy', (
    tester,
  ) async {
    final pending = Completer<Revision3VoiceTargetPublication>();
    Revision3VoiceTargetTechnicalPlan? received;
    final service = Revision3VoiceTargetAuthoringService(
      loadContentIndex: () async => revision3VoiceContentIndexFixture(),
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) {
            received = plan;
            return pending.future;
          },
    );

    await _openDialog(
      tester,
      service,
      initialLineId: revision3VoiceContentLineId,
      initialLocale: 'de',
      fixedContext: true,
      copy: Revision3VoiceTargetDialogCopy.german,
    );

    expect(find.text('Installiertes Voice-Ziel auflösen'), findsOneWidget);
    expect(find.text('Voice-Sprache: de'), findsOneWidget);
    expect(find.text('Aktuelles Ziel: nicht aufgelöst'), findsOneWidget);
    expect(find.text('Speichert Nachweis im Projekt'), findsOneWidget);
    expect(find.text('Keine Bereitstellung'), findsOneWidget);
    expect(find.text('Kein Treffer wird erfunden'), findsOneWidget);
    expect(find.text('Installiertes Ziel auflösen'), findsOneWidget);
    expect(find.text('Abbrechen'), findsOneWidget);
    expect(find.text(revision3VoiceContentLineId), findsNothing);
    expect(find.text(revision3VoiceContentSlotId), findsNothing);

    await tester.tap(find.byKey(const Key('revision3-voice-target-submit')));
    await tester.pump();
    await tester.pump();
    expect(find.text('Ziel wird aufgelöst …'), findsOneWidget);

    pending.complete(
      _publication(
        received!,
        projectId: revision3VoiceContentProjectId,
        revision: 8,
      ),
    );
    await tester.pumpAndSettle();

    expect(received?.lineId, revision3VoiceContentLineId);
    expect(received?.locale, 'de');
    expect(
      find.byKey(const Key('revision3-voice-target-dialog')),
      findsNothing,
    );
  });

  testWidgets('German search copy preserves fail-closed selection clearing', (
    tester,
  ) async {
    final service = Revision3VoiceTargetAuthoringService(
      loadContentIndex: () async => revision3VoiceContentIndexFixture(),
      publishTechnicalPlan: _unexpectedPublish,
    );

    await _openDialog(
      tester,
      service,
      copy: Revision3VoiceTargetDialogCopy.german,
    );

    expect(find.text('Dialogzeile mit vorhandenem Voice-Slot'), findsOneWidget);
    expect(find.text('Nach Sprecher oder Zeilenname suchen'), findsOneWidget);
    expect(find.text('Sprache des vorhandenen Voice-Slots'), findsOneWidget);

    await _chooseAsghanLine(tester);
    expect(_submitButton(tester).onPressed, isNotNull);
    expect(find.text('Aktuelles Ziel: nicht aufgelöst'), findsOneWidget);

    await tester.enterText(
      find.byKey(const Key('revision3-voice-target-line-search')),
      'Andere Zeile',
    );
    await tester.pumpAndSettle();

    expect(_submitButton(tester).onPressed, isNull);
    expect(find.text('Installiertes Ziel auflösen'), findsOneWidget);
    expect(find.text(revision3VoiceContentLineId), findsNothing);
  });

  testWidgets('German native errors stay safe and retryable', (tester) async {
    final service = Revision3VoiceTargetAuthoringService(
      loadContentIndex: () async => revision3VoiceContentIndexFixture(),
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async => throw const ModFfiException(
            command: 'authoring_revision3_voice_target',
            code: 'AUTHORING_REVISION3_VOICE_TARGET_ARCHIVE_UNAVAILABLE',
            message:
                r'C:\private\install\Voice.pak konnte nicht geöffnet werden',
          ),
    );

    await _openDialog(
      tester,
      service,
      copy: Revision3VoiceTargetDialogCopy.german,
    );
    await _chooseAsghanLine(tester);
    await tester.tap(find.byKey(const Key('revision3-voice-target-submit')));
    await tester.pumpAndSettle();

    expect(
      find.textContaining(
        'Das installierte Voice-Archiv für diese Sprache ist nicht verfügbar',
      ),
      findsOneWidget,
    );
    expect(find.textContaining(r'C:\private'), findsNothing);
    expect(_submitButton(tester).onPressed, isNotNull);
    expect(find.text('Abbrechen'), findsOneWidget);
  });

  testWidgets('German stale checkpoint is terminal and requires closing', (
    tester,
  ) async {
    var loads = 0;
    var publishes = 0;
    final service = Revision3VoiceTargetAuthoringService(
      loadContentIndex: () async {
        loads++;
        return revision3VoiceContentIndexFixture(revision: loads == 1 ? 7 : 8);
      },
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            publishes++;
            return _publication(
              plan,
              projectId: expectedProjectId,
              revision: expectedProjectRevision + 1,
            );
          },
    );

    await _openDialog(
      tester,
      service,
      copy: Revision3VoiceTargetDialogCopy.german,
    );
    await _chooseAsghanLine(tester);
    await tester.tap(find.byKey(const Key('revision3-voice-target-submit')));
    await tester.pumpAndSettle();

    expect(publishes, 0);
    expect(
      find.textContaining(
        'Das verwaltete Projekt wurde geändert, während dieses Fenster geöffnet war',
      ),
      findsOneWidget,
    );
    expect(_submitButton(tester).onPressed, isNull);
    expect(find.text('Schließen'), findsOneWidget);
    expect(find.text('Abbrechen'), findsNothing);
  });
}

Future<void> _openDialog(
  WidgetTester tester,
  Revision3VoiceTargetAuthoringService service, {
  ValueChanged<Revision3VoiceTargetPublication?>? onResult,
  String? initialLineId,
  String? initialLocale,
  bool fixedContext = false,
  Revision3VoiceTargetDialogCopy copy = Revision3VoiceTargetDialogCopy.english,
}) async {
  tester.view.physicalSize = const Size(1200, 900);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.resetPhysicalSize);
  addTearDown(tester.view.resetDevicePixelRatio);
  await tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: Builder(
          builder: (context) => FilledButton(
            key: const Key('open-voice-target'),
            onPressed: () async {
              final result = await showDialog<Revision3VoiceTargetPublication>(
                context: context,
                builder: (_) => Revision3VoiceTargetDialog(
                  service: service,
                  initialLineId: initialLineId,
                  initialLocale: initialLocale,
                  fixedContext: fixedContext,
                  copy: copy,
                ),
              );
              onResult?.call(result);
            },
            child: const Text('Open'),
          ),
        ),
      ),
    ),
  );
  await tester.tap(find.byKey(const Key('open-voice-target')));
  await tester.pumpAndSettle();
}

Future<void> _chooseAsghanLine(WidgetTester tester) async {
  await tester.enterText(
    find.byKey(const Key('revision3-voice-target-line-search')),
    'Asghan',
  );
  await tester.pumpAndSettle();
  final results = find.byKey(const Key('revision3-voice-target-line-results'));
  final choices = find.descendant(of: results, matching: find.byType(ListTile));
  expect(choices, findsOneWidget);
  await tester.tap(choices);
  await tester.pumpAndSettle();
}

FilledButton _submitButton(WidgetTester tester) => tester.widget<FilledButton>(
  find.byKey(const Key('revision3-voice-target-submit')),
);

Future<Revision3VoiceTargetPublication> _unexpectedPublish({
  required String expectedProjectId,
  required int expectedProjectRevision,
  required Revision3VoiceTargetTechnicalPlan plan,
}) => throw StateError('No target publication was expected.');

Revision3VoiceTargetPublication _publication(
  Revision3VoiceTargetTechnicalPlan plan, {
  required String projectId,
  required int revision,
}) => Revision3VoiceTargetPublication(
  projectId: projectId,
  projectRevision: revision,
  lineId: plan.lineId,
  slotId: plan.slotId,
  locale: plan.locale,
  locId: plan.locId,
  resolution: AuthoringRevision3VoiceTargetResolutionState.resolved,
  matchCount: 1,
);
