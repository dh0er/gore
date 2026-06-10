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

void main() {
  testWidgets('renders groups as separate cards and saves dirty rows as one batch',
      (tester) async {
    final saved = <List<TypedValueEdit>>[];
    final attributes = [
      _attribute('MaxHealth', '/Script/G1R.AttributeSet_Health', 64),
      _attribute('Critical_OneHand', '/Script/G1R.AttributeSet_Critical', 3),
      _attribute('Swampweed', '/Script/G1R.AttributeSet_Drugs', 0),
    ];

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SingleChildScrollView(
            child: HeroStatsCard(
              load: () async => HeroAttributesResult(attributes: attributes),
              save: (edits) async {
                saved.add(edits);
                return true;
              },
              editable: true,
              reloadKey: 'save-1',
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    // No outer 'Hero stats' wrapper title.
    expect(find.text('Hero stats'), findsNothing);

    // Group titles rendered as card headers.
    expect(find.text('Main stats'), findsOneWidget);
    expect(find.text('Combat skills'), findsOneWidget);
    expect(find.text('Advanced'), findsOneWidget);
    expect(find.text('MaxHealth'), findsOneWidget);
    // Advanced group is collapsed: its row is not visible.
    expect(find.text('Swampweed'), findsNothing);

    // Each non-advanced group title lives inside its own Card.
    expect(
      find.ancestor(of: find.text('Main stats'), matching: find.byType(Card)),
      findsOneWidget,
    );
    expect(
      find.ancestor(
        of: find.text('Combat skills'),
        matching: find.byType(Card),
      ),
      findsOneWidget,
    );

    // Edit MaxHealth base value, then save.
    final baseField = find.widgetWithText(TextField, 'MaxHealth base');
    await tester.enterText(baseField, '99');
    await tester.pump();
    await tester.tap(find.byTooltip('Save hero stats'));
    await tester.pumpAndSettle();

    expect(saved, hasLength(1));
    expect(saved.single, hasLength(1));
    final edit = saved.single.single;
    expect(edit.path.last, 'BaseValue');
    expect(edit.path[edit.path.length - 2], '{MaxHealth}');
    expect(edit.value, 99);
  });

  testWidgets('expanding advanced shows remaining attributes', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SingleChildScrollView(
            child: HeroStatsCard(
              load: () async => HeroAttributesResult(attributes: [
                _attribute('Swampweed', '/Script/G1R.AttributeSet_Drugs', 0),
              ]),
              save: (_) async => true,
              editable: true,
              reloadKey: 'save-1',
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    // Advanced group is its own Card.
    expect(
      find.ancestor(of: find.text('Advanced'), matching: find.byType(Card)),
      findsOneWidget,
    );

    await tester.tap(find.text('Advanced'));
    await tester.pumpAndSettle();

    expect(find.text('Swampweed'), findsOneWidget);
  });

  testWidgets('shows load error inline', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: HeroStatsCard(
            load: () async =>
                const HeroAttributesResult(error: 'decode failed'),
            save: (_) async => true,
            editable: true,
            reloadKey: 'save-1',
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.textContaining('decode failed'), findsOneWidget);
  });

  testWidgets('load error renders fallback when provided', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: HeroStatsCard(
            load: () async =>
                const HeroAttributesResult(error: 'decode failed'),
            save: (_) async => true,
            editable: true,
            reloadKey: 'save-1',
            fallback: const Text('legacy editor'),
          ),
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
      MaterialApp(
        home: Scaffold(
          body: HeroStatsCard(
            load: () async => const HeroAttributesResult(attributes: []),
            save: (_) async => true,
            editable: true,
            reloadKey: 'save-1',
            fallback: const Text('legacy editor'),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('legacy editor'), findsOneWidget);
    // The fallback editor has its own save affordances; the hero-stats save
    // button is not rendered at all in this state.
    expect(find.byTooltip('Save hero stats'), findsNothing);
  });

  testWidgets('reloadKey change refreshes row values', (tester) async {
    var loadValue = 64.0;
    var reloadKey = Object();

    Widget buildCard() => MaterialApp(
          home: Scaffold(
            body: SingleChildScrollView(
              child: HeroStatsCard(
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
            ),
          ),
        );

    await tester.pumpWidget(buildCard());
    await tester.pumpAndSettle();

    // First load: both base and current fields show '64'.
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

  testWidgets('collapsed advanced edits are still saved consistently',
      (tester) async {
    final saved = <List<TypedValueEdit>>[];

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SingleChildScrollView(
            child: HeroStatsCard(
              load: () async => HeroAttributesResult(
                attributes: [
                  _attribute(
                    'Swampweed',
                    '/Script/G1R.AttributeSet_Drugs',
                    0,
                  ),
                ],
              ),
              save: (edits) async {
                saved.add(edits);
                return true;
              },
              editable: true,
              reloadKey: 'save-1',
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    // Expand Advanced tile and enter a value.
    await tester.tap(find.text('Advanced'));
    await tester.pumpAndSettle();

    await tester.enterText(
      find.widgetWithText(TextField, 'Swampweed base'),
      '99',
    );
    await tester.pump();

    // Collapse the tile, then re-expand; maintainState keeps the text.
    await tester.tap(find.text('Advanced'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Advanced'));
    await tester.pumpAndSettle();

    expect(
      tester
          .widget<TextField>(find.widgetWithText(TextField, 'Swampweed base'))
          .controller!
          .text,
      '99',
    );

    // Save and assert the single edit has value 99.
    await tester.tap(find.byTooltip('Save hero stats'));
    await tester.pumpAndSettle();

    expect(saved, hasLength(1));
    expect(saved.single, hasLength(1));
    expect(saved.single.single.value, 99);
  });

  testWidgets('reverting an edit to the original value saves nothing',
      (tester) async {
    var saveCalled = false;

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SingleChildScrollView(
            child: HeroStatsCard(
              load: () async => HeroAttributesResult(
                attributes: [
                  _attribute(
                    'MaxHealth',
                    '/Script/G1R.AttributeSet_Health',
                    64,
                  ),
                ],
              ),
              save: (_) async {
                saveCalled = true;
                return true;
              },
              editable: true,
              reloadKey: 'save-1',
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    // Change the field, then revert it back to the original value.
    final field = find.widgetWithText(TextField, 'MaxHealth base');
    await tester.enterText(field, '99');
    await tester.pump();
    await tester.enterText(field, '64');
    await tester.pump();

    await tester.tap(find.byTooltip('Save hero stats'));
    await tester.pumpAndSettle();

    expect(saveCalled, isFalse);
  });

  testWidgets('save validation error keeps typed editors over the fallback',
      (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SingleChildScrollView(
            child: HeroStatsCard(
              load: () async => HeroAttributesResult(
                attributes: [
                  _attribute(
                    'MaxHealth',
                    '/Script/G1R.AttributeSet_Health',
                    64,
                  ),
                ],
              ),
              save: (_) async => true,
              editable: true,
              reloadKey: 'save-1',
              fallback: const Text('legacy editor'),
            ),
          ),
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
