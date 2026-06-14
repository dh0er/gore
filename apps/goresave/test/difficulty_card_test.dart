import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/editor_notifier.dart';
import 'package:goresave/features/editor/ui/difficulty_card.dart';

/// Minimal core stub: every command resolves to an empty ok response so the
/// EditorNotifier constructor (which fires refresh + checkCodec) settles
/// without errors. The Difficulty card never issues a write in this test — it
/// only reads notifier.state and registers/clears the pending difficulty edit —
/// so no command needs real data.
class _StubCoreService implements GoresaveCoreService {
  @override
  bool get isAvailable => true;

  @override
  String get description => 'stub';

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    return {'ok': true, 'data': const <String, Object?>{}};
  }
}

/// A SaveInspection seeded with a Custom-preset difficulty block. Custom maps
/// the level suffixes to labels: _Custom→Custom, _Hard→Hard, _Standard→Gothic.
SaveInspection _customInspection() {
  return SaveInspection.fromJson({
    'format': 'G1R',
    'path': r'C:\tmp\saves\G1R-001.sav',
    'size': 1024,
    'sha1': 'abc',
    'difficulty': {
      'preset': 'EDifficulty_Custom',
      'combat': 'ECombat_Hard',
      'resources': 'EResources_Standard',
      'progression': 'EProgression_Easy',
      'flowHelper': true,
      'permadeath': true,
    },
  });
}

/// Resolve a SwitchListTile by its title text.
SwitchListTile _switchByTitle(WidgetTester tester, String title) {
  return tester
      .widgetList<SwitchListTile>(find.byType(SwitchListTile))
      .firstWhere((s) => (s.title as Text).data == title);
}

/// The preset SegmentedButton is the only one carrying a 'Custom' segment.
Finder _presetPicker() => find.byWidgetPredicate(
  (w) =>
      w is SegmentedButton<String> &&
      w.segments.any((s) => (s.label as Text).data == 'Custom'),
);

void main() {
  late EditorNotifier notifier;

  setUp(() {
    notifier = EditorNotifier(
      _StubCoreService(),
      saveDir: r'C:\tmp\saves',
      codecHostPath: r'C:\tools\codec.exe',
      gameExePath: r'C:\games\G1R-Win64-Shipping.exe',
    );
  });

  Future<void> pumpCard(
    WidgetTester tester, {
    ProfileSummary? profile,
    SaveInspection? inspection,
    bool canCompress = true,
  }) async {
    await tester.binding.setSurfaceSize(const Size(900, 1200));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SingleChildScrollView(
            child: DifficultyCard(
              inspection: inspection ?? _customInspection(),
              notifier: notifier,
              profile: profile,
              canCompress: canCompress,
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    // The card now opens EXPANDED by default — no need to tap to reveal it.
  }

  const profile = ProfileSummary(profileId: 1, profileName: 'Nameless Hero');

  testWidgets('the card is expanded by default (form visible on load)', (
    tester,
  ) async {
    await pumpCard(tester);
    // The preset selector (and its 'Custom' segment) is visible without any tap.
    expect(_presetPicker(), findsOneWidget);
    expect(find.text('Combat'), findsOneWidget);
  });

  testWidgets('the card has no own Save/Reset buttons', (tester) async {
    await pumpCard(tester, profile: profile);
    // No card-local Save/Reset (those live on the global toolbar now).
    expect(find.widgetWithText(FilledButton, 'Save'), findsNothing);
    expect(find.widgetWithText(OutlinedButton, 'Reset'), findsNothing);
  });

  testWidgets(
    'Custom preset enables Permadeath and all three level pickers',
    (tester) async {
      await pumpCard(tester);

      expect(_switchByTitle(tester, 'Permadeath').onChanged, isNotNull);
      for (final label in ['Combat', 'Resources', 'Progression']) {
        expect(find.text(label), findsOneWidget);
      }
      final levelPickers = tester
          .widgetList<SegmentedButton<String>>(
            find.byType(SegmentedButton<String>),
          )
          .where(
            (b) => b.segments.every((s) => (s.label as Text).data != 'Custom'),
          )
          .toList();
      expect(levelPickers, hasLength(3));
      for (final picker in levelPickers) {
        expect(picker.onSelectionChanged, isNotNull);
      }
    },
  );

  testWidgets(
    'Novice preset disables Permadeath and all three level pickers',
    (tester) async {
      await pumpCard(tester);

      await tester.tap(
        find.descendant(of: _presetPicker(), matching: find.text('Novice')),
      );
      await tester.pumpAndSettle();

      final perma = _switchByTitle(tester, 'Permadeath');
      expect(perma.onChanged, isNull, reason: 'Novice disables Permadeath');
      expect(perma.value, isFalse, reason: 'Novice forces Permadeath off');

      final levelPickers = tester
          .widgetList<SegmentedButton<String>>(
            find.byType(SegmentedButton<String>),
          )
          .where(
            (b) => b.segments.every((s) => (s.label as Text).data != 'Custom'),
          )
          .toList();
      expect(levelPickers, hasLength(3));
      for (final picker in levelPickers) {
        expect(picker.onSelectionChanged, isNull);
      }

      expect(
        _switchByTitle(tester, 'Close Combat Flow Helper').onChanged,
        isNotNull,
      );
    },
  );

  testWidgets(
    'changing a control registers a pending difficulty edit the GLOBAL count '
    'reflects',
    (tester) async {
      await pumpCard(tester, profile: profile);

      // Nothing pending initially.
      expect(notifier.state.pendingDifficulty, isNull);
      expect(notifier.state.pendingEditCount, 0);

      // Switch preset Custom → Novice: a real difficulty change.
      await tester.tap(
        find.descendant(of: _presetPicker(), matching: find.text('Novice')),
      );
      await tester.pumpAndSettle();

      // A pending difficulty edit is now registered and the GLOBAL count/badge
      // reflect it.
      expect(notifier.state.pendingDifficulty, isNotNull);
      expect(notifier.state.pendingEditCount, 1);
      expect(notifier.state.hasUnsavedEdits, isTrue);
      expect(notifier.state.pendingDifficulty!.difficulty['preset'], 'Novice');
      // The card shows the 'Unsaved' badge.
      expect(find.text('Unsaved'), findsOneWidget);
    },
  );

  testWidgets(
    'reverting a control back to the stored value clears the pending edit',
    (tester) async {
      await pumpCard(tester, profile: profile);

      // Stored flowHelper is true. Toggle it off (pending), then back on.
      final flowFinder = find.ancestor(
        of: find.text('Close Combat Flow Helper'),
        matching: find.byType(SwitchListTile),
      );
      await tester.tap(flowFinder);
      await tester.pumpAndSettle();
      expect(notifier.state.pendingDifficulty, isNotNull);

      await tester.tap(flowFinder);
      await tester.pumpAndSettle();
      // Back at the stored value with no propagation → no pending work.
      expect(notifier.state.pendingDifficulty, isNull);
      expect(notifier.state.pendingEditCount, 0);
      expect(find.text('Unsaved'), findsNothing);
    },
  );

  testWidgets(
    'Custom: a changed sub-level stays pending across a later _apply '
    '(propagation toggle) — compared against STORED, not the draft',
    (tester) async {
      await pumpCard(tester, profile: profile);

      // Stored Custom combat is Hard. Change it to Novice → a real edit.
      // Target the Combat picker's SegmentedButton (the Row that holds the
      // 'Combat' label).
      final combatRow = find.ancestor(
        of: find.text('Combat'),
        matching: find.byType(Row),
      );
      final combatNovice = find.descendant(
        of: combatRow,
        matching: find.text('Novice'),
      );
      await tester.tap(combatNovice);
      await tester.pumpAndSettle();
      expect(notifier.state.pendingDifficulty, isNotNull);
      expect(notifier.state.pendingDifficulty!.difficulty['combat'], 'Novice');

      // Now toggle a propagation box ON then OFF. Each fires _apply. With the
      // bug, the second toggle would compare the changed combat against the
      // PENDING DRAFT (also Novice) → match → wrongly clear, dropping the
      // unsaved sub-level change. The fix compares against STORED (Hard).
      await tester.tap(find.text('Also update the profile'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Also update the profile'));
      await tester.pumpAndSettle();

      // The combat sub-level still differs from stored → still pending.
      expect(
        notifier.state.pendingDifficulty,
        isNotNull,
        reason: 'a sub-level differing from STORED must keep the edit pending',
      );
      expect(notifier.state.pendingDifficulty!.difficulty['combat'], 'Novice');
      expect(notifier.state.pendingDifficulty!.alsoProfile, isFalse);

      // Reverting combat back to STORED (Hard) with no propagation clears it.
      final combatHard = find.descendant(
        of: find.ancestor(
          of: find.text('Combat'),
          matching: find.byType(Row),
        ),
        matching: find.text('Hard'),
      );
      await tester.tap(combatHard);
      await tester.pumpAndSettle();
      expect(
        notifier.state.pendingDifficulty,
        isNull,
        reason: 'all sub-levels back to STORED + no propagation → cleared',
      );
    },
  );

  testWidgets(
    'propagation-only: ticking a box registers a pending edit (no field change)',
    (tester) async {
      await pumpCard(tester, profile: profile);

      expect(notifier.state.pendingDifficulty, isNull);

      await tester.tap(find.text('Also update the profile'));
      await tester.pumpAndSettle();

      // A ticked propagation box is work even without a field change.
      expect(notifier.state.pendingDifficulty, isNotNull);
      expect(notifier.state.pendingDifficulty!.alsoProfile, isTrue);
      expect(notifier.state.hasUnsavedEdits, isTrue);
    },
  );

  testWidgets(
    'no resolvable profile disables both propagation checkboxes',
    (tester) async {
      // profile == null models a save with no resolvable profile.
      await pumpCard(tester, profile: null);

      final boxes = tester.widgetList<CheckboxListTile>(
        find.byType(CheckboxListTile),
      );
      expect(boxes, hasLength(2));
      for (final box in boxes) {
        expect(
          box.onChanged,
          isNull,
          reason: 'Cannot propagate without a resolved profile',
        );
      }
    },
  );

  testWidgets(
    'a global Reset (pendingDifficulty cleared) makes the card show stored '
    'values again',
    (tester) async {
      await pumpCard(tester, profile: profile);

      // Make the draft Novice.
      await tester.tap(
        find.descendant(of: _presetPicker(), matching: find.text('Novice')),
      );
      await tester.pumpAndSettle();
      expect(notifier.state.pendingDifficulty, isNotNull);

      // Simulate the global Reset clearing the pending difficulty, then a
      // parent rebuild of the card (same save).
      notifier.clearPendingDifficulty();
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: SingleChildScrollView(
              child: DifficultyCard(
                inspection: _customInspection(),
                notifier: notifier,
                profile: profile,
                canCompress: true,
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();

      // The card renders the stored Custom preset again.
      final presetPicker = tester.widget<SegmentedButton<String>>(
        _presetPicker(),
      );
      expect(presetPicker.selected, {'Custom'});
      expect(find.text('Unsaved'), findsNothing);
    },
  );

  testWidgets(
    'a non-blocking codec hint shows when an edit is pending but the codec is '
    'not compress-ready',
    (tester) async {
      await pumpCard(tester, profile: profile, canCompress: false);

      // No hint until there is pending work.
      expect(find.textContaining('verified G1R codec host'), findsNothing);

      await tester.tap(
        find.descendant(of: _presetPicker(), matching: find.text('Novice')),
      );
      await tester.pumpAndSettle();

      // The hint appears, but the controls remain enabled — the global Save
      // surfaces the actual error; the card no longer gates.
      expect(find.textContaining('verified G1R codec host'), findsOneWidget);
      expect(notifier.state.pendingDifficulty, isNotNull);
    },
  );
}
