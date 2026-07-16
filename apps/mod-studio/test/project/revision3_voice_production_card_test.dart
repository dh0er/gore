import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_voice_authoring.dart';
import 'package:gore_mod/project/revision3_voice_production_card.dart';

import '../support/revision3_voice_content_fixture.dart';

void main() {
  testWidgets(
    'loading, error, and unavailable states stay generic and closed',
    (tester) async {
      final line = _line(existingDeSlot: false);

      await _pumpCard(
        tester,
        line: line,
        locale: 'de',
        slotExpected: false,
        loading: true,
        onAddTake: () {},
      );
      expect(
        find.byKey(const Key('revision3-voice-production-loading')),
        findsOneWidget,
      );
      expect(_actionFinder, findsNothing);

      await _pumpCard(
        tester,
        line: line,
        locale: 'de',
        slotExpected: false,
        error: StateError(
          '${revision3VoiceContentLineId}_C:\\private\\recording.ogg',
        ),
        onAddTake: () {},
      );
      expect(
        find.byKey(const Key('revision3-voice-production-error')),
        findsOneWidget,
      );
      expect(find.text('Voice context unavailable'), findsOneWidget);
      expect(find.textContaining(revision3VoiceContentLineId), findsNothing);
      expect(find.textContaining(r'C:\private\recording.ogg'), findsNothing);
      expect(_actionFinder, findsNothing);

      await _pumpCard(
        tester,
        line: null,
        locale: null,
        slotExpected: false,
        onAddTake: () {},
        onManageTakes: () {},
        onResolveTarget: () {},
      );
      expect(
        find.byKey(const Key('revision3-voice-production-unavailable')),
        findsOneWidget,
      );
      expect(find.text('Select a dialog line and language'), findsOneWidget);
      expect(_actionFinder, findsNothing);
    },
  );

  testWidgets('no-slot state offers only the exact gated add action', (
    tester,
  ) async {
    var addCalls = 0;
    var manageCalls = 0;
    var resolveCalls = 0;
    final line = _line(existingDeSlot: false);

    await _pumpCard(
      tester,
      line: line,
      locale: 'de',
      slotExpected: false,
      onAddTake: () => addCalls++,
      onManageTakes: () => manageCalls++,
      onResolveTarget: () => resolveCalls++,
    );

    expect(
      find.byKey(const Key('revision3-voice-production-no-slot')),
      findsOneWidget,
    );
    expect(find.text('Asghan — Mine entrance question'), findsOneWidget);
    expect(find.text('Language: de'), findsOneWidget);
    expect(
      find.text('Add a recording to create this language\'s Voice setup.'),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-voice-production-add')),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-voice-production-manage')),
      findsNothing,
    );
    expect(
      find.byKey(const Key('revision3-voice-production-resolve')),
      findsNothing,
    );

    await tester.tap(find.byKey(const Key('revision3-voice-production-add')));
    await tester.pump();
    expect((addCalls, manageCalls, resolveCalls), (1, 0, 0));
  });

  testWidgets(
    'intact slot shows production facts, next step, actions, and no identities',
    (tester) async {
      var addCalls = 0;
      var manageCalls = 0;
      var resolveCalls = 0;
      final line = _line(
        existingDeSlot: true,
        candidateCount: 2,
        selected: true,
        targetResolution: 'resolved',
      );
      final summary = line.slotSummaryForLocale('de')!;

      await _pumpCard(
        tester,
        line: line,
        locale: 'de',
        slotExpected: true,
        onAddTake: () => addCalls++,
        onManageTakes: () => manageCalls++,
        onResolveTarget: () => resolveCalls++,
      );

      expect(
        find.byKey(const Key('revision3-voice-production-intact')),
        findsOneWidget,
      );
      expect(find.text('2 takes'), findsOneWidget);
      expect(find.text('1 Approved'), findsOneWidget);
      expect(find.text('Target: Resolved'), findsOneWidget);
      expect(find.text('Asghan take · 1 of 2'), findsOneWidget);
      expect(find.text('Approved'), findsWidgets);
      expect(
        find.text(
          'Review exact Voice readiness in Validate & Test before building.',
        ),
        findsOneWidget,
      );
      expect(find.textContaining(revision3VoiceContentProjectId), findsNothing);
      expect(
        find.textContaining(revision3VoiceContentLocalizationId),
        findsNothing,
      );
      expect(find.textContaining(revision3VoiceContentLineId), findsNothing);
      expect(find.textContaining(revision3VoiceContentSlotId), findsNothing);
      for (final take in summary.candidates) {
        expect(find.textContaining(take.id), findsNothing);
      }
      expect(find.textContaining('Preview'), findsNothing);
      expect(find.byIcon(Icons.play_arrow), findsNothing);

      await tester.tap(find.byKey(const Key('revision3-voice-production-add')));
      await tester.tap(
        find.byKey(const Key('revision3-voice-production-manage')),
      );
      await tester.tap(
        find.byKey(const Key('revision3-voice-production-resolve')),
      );
      await tester.pump();
      expect((addCalls, manageCalls, resolveCalls), (1, 1, 1));
    },
  );

  for (final target in const <(String, String, String)>[
    (
      'unresolved',
      'Target: Unresolved',
      'Resolve the installed Voice target for this dialog line and language.',
    ),
    (
      'ambiguous',
      'Target: Ambiguous',
      'Resolve the installed Voice target for this dialog line and language.',
    ),
    (
      'resolved',
      'Target: Resolved',
      'Review exact Voice readiness in Validate & Test before building.',
    ),
  ]) {
    testWidgets('intact slot presents ${target.$1} target honestly', (
      tester,
    ) async {
      await _pumpCard(
        tester,
        line: _line(
          existingDeSlot: true,
          candidateCount: 1,
          selected: true,
          targetResolution: target.$1,
        ),
        locale: 'de',
        slotExpected: true,
      );

      expect(find.text(target.$2), findsOneWidget);
      expect(find.text(target.$3), findsOneWidget);
    });
  }

  testWidgets('intact slot explains missing approval and selection', (
    tester,
  ) async {
    await _pumpCard(
      tester,
      line: _line(existingDeSlot: true, candidateCount: 2, selected: false),
      locale: 'de',
      slotExpected: true,
      onManageTakes: () {},
    );

    expect(find.text('2 takes'), findsOneWidget);
    expect(find.text('0 Approved'), findsOneWidget);
    expect(find.text('No take selected'), findsOneWidget);
    expect(
      find.text(
        'Review a take, mark it Approved, and then select it in Manage takes.',
      ),
      findsOneWidget,
    );
  });

  testWidgets('expected but unavailable slot is explicit and actionless', (
    tester,
  ) async {
    await _pumpCard(
      tester,
      line: _line(existingDeSlot: false),
      locale: 'de',
      slotExpected: true,
      onAddTake: () {},
      onManageTakes: () {},
      onResolveTarget: () {},
    );

    expect(
      find.byKey(const Key('revision3-voice-production-unsafe')),
      findsOneWidget,
    );
    expect(find.text('Voice setup needs attention'), findsOneWidget);
    expect(_actionFinder, findsNothing);
  });

  testWidgets('all intact actions wrap without overflow at narrow width', (
    tester,
  ) async {
    await _setSurface(tester, const Size(280, 900));
    await _pumpCard(
      tester,
      line: _line(
        existingDeSlot: true,
        candidateCount: 1,
        selected: true,
        targetResolution: 'resolved',
      ),
      locale: 'de',
      slotExpected: true,
      onAddTake: () {},
      onManageTakes: () {},
      onResolveTarget: () {},
    );

    final add = find.byKey(const Key('revision3-voice-production-add'));
    final manage = find.byKey(const Key('revision3-voice-production-manage'));
    final resolve = find.byKey(const Key('revision3-voice-production-resolve'));
    expect(add, findsOneWidget);
    expect(manage, findsOneWidget);
    expect(resolve, findsOneWidget);
    expect(
      tester.getTopLeft(resolve).dy,
      greaterThan(tester.getTopLeft(add).dy),
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('German copy keeps the same exact production facts', (
    tester,
  ) async {
    await _pumpCard(
      tester,
      line: _line(
        existingDeSlot: true,
        candidateCount: 1,
        selected: true,
        targetResolution: 'resolved',
      ),
      locale: 'de',
      slotExpected: true,
      copy: Revision3VoiceProductionCardCopy.german,
    );

    expect(find.text('Voice-Produktion'), findsOneWidget);
    expect(find.text('Sprache: de'), findsOneWidget);
    expect(find.text('1 Aufnahme'), findsOneWidget);
    expect(find.text('Ziel: Aufgelöst'), findsOneWidget);
    expect(find.text('Freigegeben'), findsWidgets);
  });
}

Finder get _actionFinder =>
    find.byKey(const Key('revision3-voice-production-actions'));

Revision3VoiceDialogLineChoice _line({
  required bool existingDeSlot,
  int candidateCount = 0,
  bool selected = false,
  String targetResolution = 'unresolved',
}) {
  final catalog = Revision3VoiceCatalog.fromContentIndex(
    revision3VoiceContentIndexFixture(
      existingDeSlot: existingDeSlot,
      existingSlotCandidateCount: candidateCount,
      existingSlotHasSelectedTake: selected,
      existingSlotTargetResolution: targetResolution,
    ),
  );
  return catalog.line(revision3VoiceContentLineId)!;
}

Future<void> _pumpCard(
  WidgetTester tester, {
  required Revision3VoiceDialogLineChoice? line,
  required String? locale,
  required bool slotExpected,
  bool loading = false,
  Object? error,
  VoidCallback? onAddTake,
  VoidCallback? onManageTakes,
  VoidCallback? onResolveTarget,
  Revision3VoiceProductionCardCopy copy =
      Revision3VoiceProductionCardCopy.english,
}) => tester.pumpWidget(
  MaterialApp(
    home: Scaffold(
      body: Align(
        alignment: Alignment.topCenter,
        child: Revision3VoiceProductionCard(
          line: line,
          locale: locale,
          slotExpected: slotExpected,
          loading: loading,
          error: error,
          onAddTake: onAddTake,
          onManageTakes: onManageTakes,
          onResolveTarget: onResolveTarget,
          copy: copy,
        ),
      ),
    ),
  ),
);

Future<void> _setSurface(WidgetTester tester, Size size) async {
  tester.view.devicePixelRatio = 1;
  tester.view.physicalSize = size;
  addTearDown(() {
    tester.view.resetDevicePixelRatio();
    tester.view.resetPhysicalSize();
  });
}
