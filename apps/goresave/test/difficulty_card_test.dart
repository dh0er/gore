import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/editor_notifier.dart';
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

  Future<void> pumpCard(WidgetTester tester, {ProfileSummary? profile}) async {
    await tester.binding.setSurfaceSize(const Size(900, 1200));
    addTearDown(() => tester.binding.setSurfaceSize(null));
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
    // The card opens collapsed; expand it to reveal the editing form.
    await tester.tap(find.text('Difficulty'));
    await tester.pumpAndSettle();
  }

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
}
