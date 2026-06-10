import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/hero_attributes.dart';
import 'package:goresave/features/editor/ui/hero_stats_card.dart';

HeroAttribute _attribute(String id, String setClass, double value) {
  final prefix = [
    'm_GenericData',
    '{CharacterStates}',
    'AnyCharacterType',
    'AttributesByGlobalId',
    '{Hero}',
    'AttributeSetsByClass',
    '{$setClass}',
    'Attributes',
    '{$id}',
  ];
  return HeroAttribute(
    id: id,
    setClass: setClass,
    basePath: [...prefix, 'BaseValue'],
    currentPath: [...prefix, 'CurrentValue'],
    baseValue: value,
    currentValue: value,
  );
}

// Wrap in a constrained box so LayoutBuilder in rows has a finite width.
Widget _wrap(Widget child) => MaterialApp(
      home: Scaffold(
        body: SizedBox(
          width: 800,
          height: 600,
          child: child,
        ),
      ),
    );

void main() {
  // ---------------------------------------------------------------------------
  // Sidebar visibility / navigation
  // ---------------------------------------------------------------------------

  testWidgets('sidebar shows only non-empty groups', (tester) async {
    final attributes = [
      _attribute('MaxHealth', '/Script/G1R.AttributeSet_Health', 64),
      _attribute('Critical_OneHand', '/Script/G1R.AttributeSet_Critical', 3),
      // No thieving, no resistances.
    ];

    await tester.pumpWidget(
      _wrap(
        HeroStatsCard(
          load: () async => HeroAttributesResult(attributes: attributes),
          save: (_) async => true,
          editable: true,
          reloadKey: 'save-1',
        ),
      ),
    );
    await tester.pumpAndSettle();

    // Sidebar entries present for non-empty groups (may also appear in the
    // detail card header, so use findsWidgets not findsOneWidget).
    expect(find.text('Main stats'), findsWidgets);
    expect(find.text('Combat skills'), findsWidgets);
    // Entries absent for empty groups.
    expect(find.text('Resistances'), findsNothing);
    expect(find.text('Thieving'), findsNothing);
    expect(find.text('Advanced'), findsNothing);
    // No outer 'Hero stats' wrapper title.
    expect(find.text('Hero stats'), findsNothing);
  });

  testWidgets('selecting a group shows its rows and hides others',
      (tester) async {
    final attributes = [
      _attribute('MaxHealth', '/Script/G1R.AttributeSet_Health', 64),
      _attribute('Critical_OneHand', '/Script/G1R.AttributeSet_Critical', 3),
    ];

    await tester.pumpWidget(
      _wrap(
        HeroStatsCard(
          load: () async => HeroAttributesResult(attributes: attributes),
          save: (_) async => true,
          editable: true,
          reloadKey: 'save-1',
        ),
      ),
    );
    await tester.pumpAndSettle();

    // Default selection is Main stats — its row is shown.
    expect(find.widgetWithText(TextField, 'MaxHealth base'), findsOneWidget);
    // Combat skills row is not shown yet.
    expect(
      find.widgetWithText(TextField, 'Critical_OneHand base'),
      findsNothing,
    );

    // Switch to Combat skills.
    await tester.tap(find.text('Combat skills'));
    await tester.pumpAndSettle();

    expect(
      find.widgetWithText(TextField, 'Critical_OneHand base'),
      findsOneWidget,
    );
    expect(find.widgetWithText(TextField, 'MaxHealth base'), findsNothing);
  });

  testWidgets('hero transform entry shown when transformCard provided',
      (tester) async {
    await tester.pumpWidget(
      _wrap(
        HeroStatsCard(
          load: () async =>
              HeroAttributesResult(attributes: [
                _attribute('MaxHealth', '/Script/G1R.AttributeSet_Health', 64),
              ]),
          save: (_) async => true,
          editable: true,
          reloadKey: 'save-1',
          transformCard: const Text('transform content'),
        ),
      ),
    );
    await tester.pumpAndSettle();

    // Sidebar entry present.
    expect(find.text('Hero transform'), findsOneWidget);
    // Detail area currently shows Main stats (default selection).
    expect(find.widgetWithText(TextField, 'MaxHealth base'), findsOneWidget);
    expect(find.text('transform content'), findsNothing);

    // Tap transform entry.
    await tester.tap(find.text('Hero transform'));
    await tester.pumpAndSettle();

    expect(find.text('transform content'), findsOneWidget);
    expect(find.widgetWithText(TextField, 'MaxHealth base'), findsNothing);
  });

  testWidgets('hero transform not shown when transformCard is null',
      (tester) async {
    await tester.pumpWidget(
      _wrap(
        HeroStatsCard(
          load: () async =>
              HeroAttributesResult(attributes: [
                _attribute('MaxHealth', '/Script/G1R.AttributeSet_Health', 64),
              ]),
          save: (_) async => true,
          editable: true,
          reloadKey: 'save-1',
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Hero transform'), findsNothing);
  });

  testWidgets('Advanced is a regular sidebar entry (no ExpansionTile)',
      (tester) async {
    await tester.pumpWidget(
      _wrap(
        HeroStatsCard(
          load: () async =>
              HeroAttributesResult(attributes: [
                _attribute(
                  'Swampweed',
                  '/Script/G1R.AttributeSet_Drugs',
                  0,
                ),
              ]),
          save: (_) async => true,
          editable: true,
          reloadKey: 'save-1',
        ),
      ),
    );
    await tester.pumpAndSettle();

    // Sidebar entry exists (may appear in sidebar AND card header).
    expect(find.text('Advanced'), findsWidgets);
    // Default: only group, so it's selected — row is immediately visible.
    expect(find.widgetWithText(TextField, 'Swampweed base'), findsOneWidget);
    // No ExpansionTile needed.
    expect(find.byType(ExpansionTile), findsNothing);
  });

  // ---------------------------------------------------------------------------
  // Cross-group save: global save batches ALL dirty fields
  // ---------------------------------------------------------------------------

  testWidgets(
      'global save batches dirty fields across different groups in one call',
      (tester) async {
    final saved = <List<TypedValueEdit>>[];
    final attributes = [
      _attribute('MaxHealth', '/Script/G1R.AttributeSet_Health', 64),
      _attribute('Critical_OneHand', '/Script/G1R.AttributeSet_Critical', 3),
    ];

    await tester.pumpWidget(
      _wrap(
        HeroStatsCard(
          load: () async => HeroAttributesResult(attributes: attributes),
          save: (edits) async {
            saved.add(edits);
            return true;
          },
          editable: true,
          reloadKey: 'save-1',
        ),
      ),
    );
    await tester.pumpAndSettle();

    // Edit MaxHealth (Main stats — default selected).
    await tester.enterText(
      find.widgetWithText(TextField, 'MaxHealth base'),
      '99',
    );
    await tester.pump();

    // Switch to Combat skills and edit there.
    await tester.tap(find.text('Combat skills'));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.widgetWithText(TextField, 'Critical_OneHand base'),
      '10',
    );
    await tester.pump();

    // Save — both edits must arrive in one batch.
    await tester.tap(find.byTooltip('Save hero stats'));
    await tester.pumpAndSettle();

    expect(saved, hasLength(1));
    expect(saved.single, hasLength(2));
    final ids = saved.single.map((e) => e.path[e.path.length - 2]).toSet();
    expect(ids, containsAll(['{MaxHealth}', '{Critical_OneHand}']));
  });

  // ---------------------------------------------------------------------------
  // Pending edit survives sidebar switch and is visible on return
  // ---------------------------------------------------------------------------

  testWidgets('pending edit still visible after switching away and back',
      (tester) async {
    final attributes = [
      _attribute('MaxHealth', '/Script/G1R.AttributeSet_Health', 64),
      _attribute('Critical_OneHand', '/Script/G1R.AttributeSet_Critical', 3),
    ];

    await tester.pumpWidget(
      _wrap(
        HeroStatsCard(
          load: () async => HeroAttributesResult(attributes: attributes),
          save: (_) async => true,
          editable: true,
          reloadKey: 'save-1',
        ),
      ),
    );
    await tester.pumpAndSettle();

    // Edit MaxHealth (Main stats).
    await tester.enterText(
      find.widgetWithText(TextField, 'MaxHealth base'),
      '77',
    );
    await tester.pump();

    // Switch to Combat skills.
    await tester.tap(find.text('Combat skills'));
    await tester.pumpAndSettle();

    // The MaxHealth field is gone from the tree now.
    expect(find.widgetWithText(TextField, 'MaxHealth base'), findsNothing);

    // Switch back to Main stats.
    await tester.tap(find.text('Main stats'));
    await tester.pumpAndSettle();

    // Pending edit '77' must be visible again.
    expect(
      tester
          .widget<TextField>(find.widgetWithText(TextField, 'MaxHealth base'))
          .controller!
          .text,
      '77',
    );
  });

  // ---------------------------------------------------------------------------
  // Save semantics
  // ---------------------------------------------------------------------------

  testWidgets('reverting an edit to the original value saves nothing',
      (tester) async {
    var saveCalled = false;

    await tester.pumpWidget(
      _wrap(
        HeroStatsCard(
          load: () async =>
              HeroAttributesResult(attributes: [
                _attribute(
                  'MaxHealth',
                  '/Script/G1R.AttributeSet_Health',
                  64,
                ),
              ]),
          save: (_) async {
            saveCalled = true;
            return true;
          },
          editable: true,
          reloadKey: 'save-1',
        ),
      ),
    );
    await tester.pumpAndSettle();

    final field = find.widgetWithText(TextField, 'MaxHealth base');
    await tester.enterText(field, '99');
    await tester.pump();
    await tester.enterText(field, '64');
    await tester.pump();

    await tester.tap(find.byTooltip('Save hero stats'));
    await tester.pumpAndSettle();

    expect(saveCalled, isFalse);
  });

  testWidgets('double-tapping save issues only one batched write',
      (tester) async {
    var saveCalls = 0;
    final gate = Completer<bool>();

    await tester.pumpWidget(
      _wrap(
        HeroStatsCard(
          load: () async =>
              HeroAttributesResult(attributes: [
                _attribute(
                  'MaxHealth',
                  '/Script/G1R.AttributeSet_Health',
                  64,
                ),
              ]),
          save: (_) {
            saveCalls++;
            return gate.future;
          },
          editable: true,
          reloadKey: 'save-1',
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.enterText(
      find.widgetWithText(TextField, 'MaxHealth base'),
      '99',
    );
    // Two taps without pumping a frame in between: the second one hits the
    // still-enabled button and must be swallowed by the re-entry guard.
    await tester.tap(find.byTooltip('Save hero stats'));
    await tester.tap(find.byTooltip('Save hero stats'));
    gate.complete(true);
    await tester.pumpAndSettle();

    expect(saveCalls, 1);
  });

  // ---------------------------------------------------------------------------
  // Load error / fallback
  // ---------------------------------------------------------------------------

  testWidgets('shows load error inline', (tester) async {
    await tester.pumpWidget(
      _wrap(
        HeroStatsCard(
          load: () async =>
              const HeroAttributesResult(error: 'decode failed'),
          save: (_) async => true,
          editable: true,
          reloadKey: 'save-1',
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.textContaining('decode failed'), findsOneWidget);
  });

  testWidgets('load error renders fallback when provided', (tester) async {
    await tester.pumpWidget(
      _wrap(
        HeroStatsCard(
          load: () async =>
              const HeroAttributesResult(error: 'decode failed'),
          save: (_) async => true,
          editable: true,
          reloadKey: 'save-1',
          fallback: const Text('legacy editor'),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.textContaining('decode failed'), findsOneWidget);
    expect(find.text('legacy editor'), findsOneWidget);
  });

  testWidgets('zero attributes render fallback without a save button',
      (tester) async {
    await tester.pumpWidget(
      _wrap(
        HeroStatsCard(
          load: () async => const HeroAttributesResult(attributes: []),
          save: (_) async => true,
          editable: true,
          reloadKey: 'save-1',
          fallback: const Text('legacy editor'),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('legacy editor'), findsOneWidget);
    // The fallback editor has its own save affordances; the hero-stats save
    // button is not rendered at all in this state.
    expect(find.byTooltip('Save hero stats'), findsNothing);
  });

  testWidgets('save validation error keeps typed editors over the fallback',
      (tester) async {
    await tester.pumpWidget(
      _wrap(
        HeroStatsCard(
          load: () async =>
              HeroAttributesResult(attributes: [
                _attribute(
                  'MaxHealth',
                  '/Script/G1R.AttributeSet_Health',
                  64,
                ),
              ]),
          save: (_) async => true,
          editable: true,
          reloadKey: 'save-1',
          fallback: const Text('legacy editor'),
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.enterText(
      find.widgetWithText(TextField, 'MaxHealth base'),
      'not a number',
    );
    await tester.pump();
    await tester.tap(find.byTooltip('Save hero stats'));
    await tester.pumpAndSettle();

    // The validation error is shown, but the typed editors stay in place —
    // only a failed load swaps in the fallback.
    expect(find.textContaining('Invalid number'), findsOneWidget);
    expect(find.text('legacy editor'), findsNothing);
    expect(find.widgetWithText(TextField, 'MaxHealth base'), findsOneWidget);
  });

  testWidgets('correcting an invalid input clears the stale validation error',
      (tester) async {
    var saveCalled = false;

    await tester.pumpWidget(
      _wrap(
        HeroStatsCard(
          load: () async =>
              HeroAttributesResult(attributes: [
                _attribute(
                  'MaxHealth',
                  '/Script/G1R.AttributeSet_Health',
                  64,
                ),
              ]),
          save: (_) async {
            saveCalled = true;
            return true;
          },
          editable: true,
          reloadKey: 'save-1',
        ),
      ),
    );
    await tester.pumpAndSettle();

    final field = find.widgetWithText(TextField, 'MaxHealth base');
    await tester.enterText(field, 'not a number');
    await tester.pump();
    await tester.tap(find.byTooltip('Save hero stats'));
    await tester.pumpAndSettle();
    expect(find.textContaining('Invalid number'), findsOneWidget);

    // Correct the field back to the original (valid, unchanged) value: the
    // save is a no-op, but the stale error must disappear.
    await tester.enterText(field, '64');
    await tester.pump();
    await tester.tap(find.byTooltip('Save hero stats'));
    await tester.pumpAndSettle();

    expect(find.textContaining('Invalid number'), findsNothing);
    expect(saveCalled, isFalse);
  });

  testWidgets('reloadKey change refreshes row values', (tester) async {
    var loadValue = 64.0;
    var reloadKey = Object();

    Widget buildCard() => _wrap(
          HeroStatsCard(
            load: () async => HeroAttributesResult(
              attributes: [
                _attribute(
                  'MaxHealth',
                  '/Script/G1R.AttributeSet_Health',
                  loadValue,
                ),
              ],
            ),
            save: (_) async => true,
            editable: true,
            reloadKey: reloadKey,
          ),
        );

    await tester.pumpWidget(buildCard());
    await tester.pumpAndSettle();

    // First load: fields show '64'.
    expect(
      find.descendant(
        of: find.byType(TextField),
        matching: find.text('64'),
      ),
      findsWidgets,
    );

    // Change reloadKey and load value — simulates a fresh SaveInspection.
    loadValue = 70.0;
    reloadKey = Object();
    await tester.pumpWidget(buildCard());
    await tester.pumpAndSettle();

    // Fields now show '70'; no stale '64' remains.
    expect(
      find.descendant(
        of: find.byType(TextField),
        matching: find.text('70'),
      ),
      findsWidgets,
    );
    expect(
      find.descendant(
        of: find.byType(TextField),
        matching: find.text('64'),
      ),
      findsNothing,
    );
  });

  // ---------------------------------------------------------------------------
  // formatHeroValue unit group
  // ---------------------------------------------------------------------------

  group('formatHeroValue', () {
    test('integer value renders without decimal point', () {
      expect(formatHeroValue(64), '64');
    });

    test('negative half renders as -0.5', () {
      expect(formatHeroValue(-0.5), '-0.5');
    });

    test('0.125 is round-trip safe (not rounded to 0.13)', () {
      expect(formatHeroValue(0.125), '0.125');
    });

    test('null renders as empty string', () {
      expect(formatHeroValue(null), '');
    });
  });
}
