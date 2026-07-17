import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_dialog_localization_authoring.dart';
import 'package:gore_mod/project/revision3_voice_authoring.dart';
import 'package:gore_mod/project/revision3_voice_production_queue.dart';
import 'package:gore_mod/project/revision3_voice_production_queue_view.dart';

import '../support/revision3_voice_content_fixture.dart';

void main() {
  testWidgets('shows both friendly work types and forwards exact identities', (
    tester,
  ) async {
    await _setSurface(tester, const Size(1000, 800));
    final queue = _queue(
      authoringLocales: const ['de', 'en'],
      localizationLocales: const ['de'],
    );
    final language = queue.items
        .whereType<Revision3VoiceMissingLanguageQueueItem>()
        .single;
    final voice = queue.items.whereType<Revision3VoiceSlotQueueItem>().single;
    (String, String, Revision3VoiceMissingLanguageQueueItem)? languageCall;
    (String, String, Revision3VoiceSlotQueueItem)? voiceCall;

    await _pumpView(
      tester,
      queue: queue,
      onAddLanguage: (choiceKey, locale, item) {
        languageCall = (choiceKey, locale, item);
      },
      onAddRecording: (lineId, locale, item) {
        voiceCall = (lineId, locale, item);
      },
    );

    expect(find.text('Work list'), findsOneWidget);
    expect(find.text('Language not added'), findsOneWidget);
    expect(find.text('Voice production'), findsOneWidget);
    expect(find.text('2 items'), findsOneWidget);
    expect(find.text('2 need action'), findsOneWidget);
    expect(find.text('0 decisions complete'), findsOneWidget);
    expect(find.textContaining('Ready'), findsNothing);
    expect(find.textContaining('runtime'), findsNothing);
    expect(find.textContaining(queue.projectId), findsNothing);
    expect(find.textContaining(voice.lineId), findsNothing);

    await tester.tap(find.text('Add language'));
    await tester.pump();
    expect(languageCall, (language.choiceStableKey, language.locale, language));

    await tester.tap(find.text('Add recording'));
    await tester.pump();
    expect(voiceCall, (voice.lineId, voice.locale, voice));
  });

  testWidgets('search and counted filters keep the work list scannable', (
    tester,
  ) async {
    await _setSurface(tester, const Size(900, 700));
    final queue = _queue(
      authoringLocales: const ['de', 'en'],
      localizationLocales: const ['de'],
    );
    await _pumpView(tester, queue: queue);

    await tester.tap(
      find.byKey(
        const Key('revision3-voice-production-queue-filter-missingLanguages'),
      ),
    );
    await tester.pump();
    expect(find.text('Add this language'), findsOneWidget);
    expect(find.text('Add a recording'), findsNothing);
    expect(find.text('Showing 1 of 2'), findsOneWidget);

    await tester.tap(
      find.byKey(const Key('revision3-voice-production-queue-filter-voice')),
    );
    await tester.pump();
    expect(find.text('Add this language'), findsNothing);
    expect(find.text('Add a recording'), findsOneWidget);

    await tester.enterText(
      find.byKey(const Key('revision3-voice-production-queue-search')),
      'nothing matches',
    );
    await tester.pump();
    expect(find.text('No matching work'), findsOneWidget);
    expect(find.text('Showing 0 of 2'), findsOneWidget);
  });

  testWidgets('one async action disables every mutation with visible reasons', (
    tester,
  ) async {
    await _setSurface(tester, const Size(700, 700));
    final queue = _queue(
      authoringLocales: const ['de'],
      localizationLocales: const ['de'],
    );
    final pending = Completer<void>();
    var calls = 0;
    await _pumpView(
      tester,
      queue: queue,
      onAddRecording: (_, _, _) {
        calls++;
        return pending.future;
      },
    );

    await tester.tap(find.text('Add recording'));
    await tester.pump();
    expect(calls, 1);
    expect(find.text('Finishing the current Voice action…'), findsOneWidget);
    expect(
      find.text('Wait for the current Voice action to finish.'),
      findsOneWidget,
    );
    expect(
      tester
          .widget<FilledButton>(
            find.ancestor(
              of: find.text('Add recording'),
              matching: find.byType(FilledButton),
            ),
          )
          .onPressed,
      isNull,
    );

    pending.complete();
    await tester.pumpAndSettle();
    expect(find.text('Finishing the current Voice action…'), findsNothing);

    await _pumpView(
      tester,
      queue: queue,
      onAddRecording: (_, _, _) => calls++,
      disabledReasonFor: (_) => 'Reopen the project before editing Voice.',
    );
    expect(
      find.text('Reopen the project before editing Voice.'),
      findsOneWidget,
    );
    await tester.tap(find.text('Add recording'));
    await tester.pump();
    expect(calls, 1);
  });

  testWidgets('compact German layout survives a short 200% viewport', (
    tester,
  ) async {
    await _setSurface(tester, const Size(360, 480));
    final queue = _queue(
      authoringLocales: const ['de'],
      localizationLocales: const ['de'],
    );
    const copy = Revision3VoiceProductionQueueCopy.german;

    await tester.pumpWidget(
      MaterialApp(
        home: MediaQuery(
          data: const MediaQueryData(textScaler: TextScaler.linear(2)),
          child: Scaffold(
            body: Revision3VoiceProductionQueueView(
              queue: queue,
              copy: copy,
              onAddRecording: (_, _, _) {},
            ),
          ),
        ),
      ),
    );
    await tester.pump();
    expect(find.text(copy.title), findsOneWidget);
    await tester.scrollUntilVisible(
      find.text(copy.addRecordingActionLabel),
      180,
      scrollable: find.byType(Scrollable).first,
    );
    expect(find.text(copy.addRecordingActionLabel), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('action is keyboard reachable and status is a live region', (
    tester,
  ) async {
    final semantics = tester.ensureSemantics();
    await _setSurface(tester, const Size(700, 700));
    final queue = _queue(
      authoringLocales: const ['de'],
      localizationLocales: const ['de'],
    );
    var calls = 0;
    await _pumpView(tester, queue: queue, onAddRecording: (_, _, _) => calls++);

    final status = find.byKey(
      const Key('revision3-voice-production-queue-status'),
    );
    expect(
      tester.getSemantics(status),
      matchesSemantics(label: 'Showing 1 of 1', isLiveRegion: true),
    );
    final item = queue.items.single;
    final itemSemantics = tester.getSemantics(
      find.byKey(Key('revision3-voice-production-queue-item-${item.key}')),
    );
    expect(itemSemantics.label, contains('Voice production'));
    expect(itemSemantics.label, isNot(contains(queue.projectId)));

    for (var index = 0; index < 7; index++) {
      await tester.sendKeyEvent(LogicalKeyboardKey.tab);
      await tester.pump();
    }
    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await tester.pump();
    expect(calls, 1);
    semantics.dispose();
  });

  testWidgets('partial retention is explained without technical details', (
    tester,
  ) async {
    await _setSurface(tester, const Size(700, 600));
    final queue = _queue(
      authoringLocales: const ['de', 'en'],
      localizationLocales: const ['de'],
      maxItems: 1,
    );
    await _pumpView(tester, queue: queue);

    expect(
      find.byKey(const Key('revision3-voice-production-queue-partial')),
      findsOneWidget,
    );
    expect(find.text('The work list is limited'), findsOneWidget);
    expect(find.textContaining('1 more item'), findsOneWidget);
    expect(find.textContaining('revision'), findsNothing);
    expect(find.textContaining('hash'), findsNothing);
  });

  testWidgets('unavailable recording evidence is disclosed truthfully', (
    tester,
  ) async {
    await _setSurface(tester, const Size(700, 600));
    final queue = _queue(
      authoringLocales: const ['de', 'en'],
      localizationLocales: const ['de'],
      includeVoiceCatalog: false,
    );
    await _pumpView(tester, queue: queue);

    expect(queue.voiceCatalogAvailable, isFalse);
    expect(
      find.byKey(
        const Key('revision3-voice-production-queue-voice-unavailable'),
      ),
      findsOneWidget,
    );
    expect(find.text('Recording work could not be checked'), findsOneWidget);
    expect(find.text('Language not added'), findsOneWidget);
  });
}

Future<void> _pumpView(
  WidgetTester tester, {
  required Revision3VoiceProductionQueue queue,
  Revision3VoiceProductionQueueCopy copy =
      const Revision3VoiceProductionQueueCopy(),
  bool busy = false,
  Revision3VoiceQueueAddLanguage? onAddLanguage,
  Revision3VoiceQueueVoiceAction? onAddRecording,
  Revision3VoiceQueueVoiceAction? onReviewAndApprove,
  Revision3VoiceQueueVoiceAction? onSelectOrRepair,
  Revision3VoiceQueueVoiceAction? onResolveTarget,
  Revision3VoiceQueueVoiceAction? onReviewChecks,
  Revision3VoiceQueueDisabledReason? disabledReasonFor,
}) async {
  await tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: Revision3VoiceProductionQueueView(
          queue: queue,
          copy: copy,
          busy: busy,
          onAddLanguage: onAddLanguage,
          onAddRecording: onAddRecording,
          onReviewAndApprove: onReviewAndApprove,
          onSelectOrRepair: onSelectOrRepair,
          onResolveTarget: onResolveTarget,
          onReviewChecks: onReviewChecks,
          disabledReasonFor: disabledReasonFor,
        ),
      ),
    ),
  );
  await tester.pump();
}

Future<void> _setSurface(WidgetTester tester, Size size) async {
  await tester.binding.setSurfaceSize(size);
  addTearDown(() => tester.binding.setSurfaceSize(null));
}

Revision3VoiceProductionQueue _queue({
  List<String> authoringLocales = const ['de', 'en'],
  List<String> localizationLocales = const ['de'],
  List<String> candidateStatuses = const [],
  bool selected = false,
  String targetResolution = 'unresolved',
  int maxItems = Revision3VoiceProductionQueue.defaultMaxItems,
  bool includeVoiceCatalog = true,
}) {
  final json = revision3VoiceContentIndexJsonFixture(
    existingDeSlot: true,
    existingSlotCandidateCount: candidateStatuses.length,
    existingSlotHasSelectedTake: selected,
    existingSlotTargetResolution: targetResolution,
  );
  json['authoring_locales'] = <Object?>[...authoringLocales]..sort();
  final localization = _entity(json, revision3VoiceContentLocalizationId);
  final localizationData = _summaryData(localization);
  localizationData['locales'] = <Object?>[...localizationLocales]..sort();

  final takes =
      (json['entities']! as List<Object?>)
          .whereType<Map<String, Object?>>()
          .where((entity) => entity['kind'] == 'voice_take')
          .toList()
        ..sort(
          (left, right) =>
              (left['id']! as String).compareTo(right['id']! as String),
        );
  for (var index = 0; index < takes.length; index++) {
    _summaryData(takes[index])['status'] = candidateStatuses[index];
  }

  final content = Revision3ContentIndex.fromJsonObject(json);
  return Revision3VoiceProductionQueue.fromCatalogs(
    localizationCatalog:
        Revision3DialogLocalizationEditCatalog.fromContentIndex(content),
    voiceCatalog: includeVoiceCatalog
        ? Revision3VoiceCatalog.fromContentIndex(content)
        : null,
    maxItems: maxItems,
  );
}

Map<String, Object?> _entity(Map<String, Object?> json, String id) =>
    (json['entities']! as List<Object?>)
        .whereType<Map<String, Object?>>()
        .singleWhere((entity) => entity['id'] == id);

Map<String, Object?> _summaryData(Map<String, Object?> entity) {
  final summary = (entity['summary']! as Map).cast<String, Object?>();
  return (summary['data']! as Map).cast<String, Object?>();
}
