import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/editor_notifier.dart';
import 'package:goresave/features/editor/domain/pending_edits.dart';
import 'package:goresave/features/editor/ui/difficulty_card.dart';

/// Minimal core stub: every command resolves to an empty ok response so the
/// EditorNotifier constructor (which fires refresh + checkCodec) settles
/// without errors. The Difficulty card never issues a write in this test — it
/// only reads notifier.state and calls setDifficultyDirty — so no command
/// needs real data.
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
              canCompress: true,
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    // The card opens collapsed; expand it to reveal the editing form.
    await tester.tap(find.text('Difficulty'));
    await tester.pumpAndSettle();
  }

  /// Resolve the Save FilledButton (the only FilledButton containing 'Save').
  FilledButton saveButton(WidgetTester tester) {
    final finder = find.ancestor(
      of: find.text('Save'),
      matching: find.byType(FilledButton),
    );
    return tester.widget<FilledButton>(finder);
  }

  /// Tap the 'Also update the profile' checkbox tile.
  Future<void> tickAlsoProfile(WidgetTester tester) async {
    await tester.tap(find.text('Also update the profile'));
    await tester.pumpAndSettle();
  }

  const profile = ProfileSummary(profileId: 1, profileName: 'Nameless Hero');

  testWidgets(
    'Custom preset enables Permadeath and all three level pickers',
    (tester) async {
      await pumpCard(tester);

      // Custom-preset save seeds Custom: pickers and Permadeath are editable.
      expect(_switchByTitle(tester, 'Permadeath').onChanged, isNotNull);
      for (final label in ['Combat', 'Resources', 'Progression']) {
        expect(find.text(label), findsOneWidget);
      }
      // The three level pickers are the SegmentedButtons whose options are
      // exactly Novice/Gothic/Hard (no 'Custom' segment). There are three of
      // them and all are enabled.
      final levelPickers = tester
          .widgetList<SegmentedButton<String>>(
            find.byType(SegmentedButton<String>),
          )
          .where((b) => b.segments.every((s) => (s.label as Text).data != 'Custom'))
          .toList();
      expect(levelPickers, hasLength(3));
      for (final picker in levelPickers) {
        expect(
          picker.onSelectionChanged,
          isNotNull,
          reason: 'Custom preset must enable every level picker',
        );
      }
    },
  );

  testWidgets(
    'Novice preset disables Permadeath and all three level pickers',
    (tester) async {
      await pumpCard(tester);

      // Switch the preset to Novice. The preset picker is the only
      // SegmentedButton that carries a 'Custom' segment, so it is uniquely
      // addressable; tap its 'Novice' option (a 'Novice' label also exists in
      // the level pickers, hence the scoped finder).
      final presetPicker = find.byWidgetPredicate(
        (w) =>
            w is SegmentedButton<String> &&
            w.segments.any((s) => (s.label as Text).data == 'Custom'),
      );
      await tester.tap(
        find.descendant(of: presetPicker, matching: find.text('Novice')),
      );
      await tester.pumpAndSettle();

      // Permadeath is disabled and forced off on Novice.
      final perma = _switchByTitle(tester, 'Permadeath');
      expect(perma.onChanged, isNull, reason: 'Novice disables Permadeath');
      expect(perma.value, isFalse, reason: 'Novice forces Permadeath off');

      // Every level picker is now locked.
      final levelPickers = tester
          .widgetList<SegmentedButton<String>>(
            find.byType(SegmentedButton<String>),
          )
          .where((b) => b.segments.every((s) => (s.label as Text).data != 'Custom'))
          .toList();
      expect(levelPickers, hasLength(3));
      for (final picker in levelPickers) {
        expect(
          picker.onSelectionChanged,
          isNull,
          reason: 'Novice must disable every level picker',
        );
      }

      // Flow Helper stays enabled regardless of preset.
      expect(
        _switchByTitle(tester, 'Close Combat Flow Helper').onChanged,
        isNotNull,
      );
    },
  );

  testWidgets(
    'propagation-only: ticking a box enables Save and marks dirty (Report #5)',
    (tester) async {
      await pumpCard(tester, profile: profile);

      // No field changed yet → Save disabled, not dirty.
      expect(saveButton(tester).onPressed, isNull);
      expect(notifier.state.difficultyDirty, isFalse);

      // Tick "Also update the profile" without changing any field.
      await tickAlsoProfile(tester);

      // Save is now enabled for the propagation-only write, and the dirty
      // signal flips so the unsaved-edits guard would prompt.
      expect(
        saveButton(tester).onPressed,
        isNotNull,
        reason: 'A ticked propagation box is work — Save must enable',
      );
      expect(
        notifier.state.difficultyDirty,
        isTrue,
        reason: 'A propagation-only intent must register as unsaved work',
      );
      expect(notifier.state.hasUnsavedEdits, isTrue);
    },
  );

  testWidgets(
    'difficulty Save is blocked while unrelated pending edits exist (Report #2)',
    (tester) async {
      // Register an unrelated pending edit before pumping the card.
      notifier.setPendingEdit(
        'publicName',
        const PendingSaveEdit(
          edits: [
            {'path': 'public.name.set', 'value': 'Renamed'},
          ],
        ),
      );
      await pumpCard(tester, profile: profile);

      // Make a real difficulty change so the only thing keeping Save disabled
      // is the blocking pending edit.
      await tester.tap(
        find.descendant(
          of: find.byWidgetPredicate(
            (w) =>
                w is SegmentedButton<String> &&
                w.segments.any((s) => (s.label as Text).data == 'Custom'),
          ),
          matching: find.text('Hard'),
        ),
      );
      await tester.pumpAndSettle();

      expect(
        saveButton(tester).onPressed,
        isNull,
        reason: 'Difficulty Save must be blocked while other edits are pending',
      );
      // The blocking hint is shown.
      expect(
        find.textContaining('Save or reset your other pending changes first'),
        findsOneWidget,
      );
    },
  );

  testWidgets(
    'dirty difficulty draft survives an incidental same-save re-inspect '
    '(Report #1 / behavior B)',
    (tester) async {
      await pumpCard(tester, profile: profile);

      // Make the draft dirty: switch preset Custom → Novice.
      await tester.tap(
        find.descendant(
          of: find.byWidgetPredicate(
            (w) =>
                w is SegmentedButton<String> &&
                w.segments.any((s) => (s.label as Text).data == 'Custom'),
          ),
          matching: find.text('Novice'),
        ),
      );
      await tester.pumpAndSettle();
      expect(notifier.state.difficultyDirty, isTrue);

      // Simulate an incidental re-inspect of the SAME save: a brand-new
      // SaveInspection instance with the same path lands in the widget tree.
      // (The notifier preserves difficultyDirty for a same-save re-inspect.)
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

      // Draft must still be Novice (preserved), and still marked unsaved.
      final presetPicker = tester.widget<SegmentedButton<String>>(
        find.byWidgetPredicate(
          (w) =>
              w is SegmentedButton<String> &&
              w.segments.any((s) => (s.label as Text).data == 'Custom'),
        ),
      );
      expect(
        presetPicker.selected,
        {'Novice'},
        reason: 'A dirty draft must survive a same-save re-inspect',
      );
      expect(notifier.state.difficultyDirty, isTrue);
    },
  );

  testWidgets(
    'stale difficultyDirty is cleared when a re-seed lands on a no-work draft '
    '(Report #1 — stuck guard)',
    (tester) async {
      await pumpCard(tester, profile: profile);

      // Drive the notifier flag dirty directly, leaving the local draft at its
      // freshly-seeded (no-work) state. This models the regression: the flag is
      // set but the card has nothing to save, so the "Unsaved" badge is hidden
      // yet the profile-switch / rescan guards stay wedged.
      notifier.setDifficultyDirty(true);
      expect(notifier.state.difficultyDirty, isTrue);

      // A re-inspect of the SAME save lands (new instance, same path). Because
      // the draft has no work, didUpdateWidget re-seeds and must schedule a
      // clear of the stale flag after the frame.
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

      // The post-frame callback has run: the stale flag is cleared, so
      // hasUnsavedEdits reflects reality and the guards are no longer stuck.
      expect(
        notifier.state.difficultyDirty,
        isFalse,
        reason: 'A re-seed to a no-work draft must clear the stale dirty flag',
      );
      expect(notifier.state.hasUnsavedEdits, isFalse);
    },
  );

  testWidgets(
    'draft re-seeds when the notifier clears difficultyDirty (discard path)',
    (tester) async {
      await pumpCard(tester, profile: profile);

      // Make the draft dirty.
      await tester.tap(
        find.descendant(
          of: find.byWidgetPredicate(
            (w) =>
                w is SegmentedButton<String> &&
                w.segments.any((s) => (s.label as Text).data == 'Custom'),
          ),
          matching: find.text('Novice'),
        ),
      );
      await tester.pumpAndSettle();
      expect(notifier.state.difficultyDirty, isTrue);

      // The discard-and-rescan path clears the dirty signal before refresh.
      // The real app rebuilds the card when notifier state changes; simulate
      // that parent rebuild by re-pumping the same widget (same save path).
      notifier.setDifficultyDirty(false);
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

      // The card re-seeds to the stored Custom preset.
      final presetPicker = tester.widget<SegmentedButton<String>>(
        find.byWidgetPredicate(
          (w) =>
              w is SegmentedButton<String> &&
              w.segments.any((s) => (s.label as Text).data == 'Custom'),
        ),
      );
      expect(
        presetPicker.selected,
        {'Custom'},
        reason: 'Clearing difficultyDirty must re-seed the draft to stored',
      );
    },
  );
}
