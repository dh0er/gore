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
  testWidgets('renders groups and saves dirty rows as one batch',
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

    // Groups: core and combat sections visible, advanced collapsed.
    expect(find.text('Main stats'), findsOneWidget);
    expect(find.text('Combat skills'), findsOneWidget);
    expect(find.text('Advanced'), findsOneWidget);
    expect(find.text('MaxHealth'), findsOneWidget);
    // Advanced group is collapsed: its row is not built.
    expect(find.text('Swampweed'), findsNothing);

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
}
