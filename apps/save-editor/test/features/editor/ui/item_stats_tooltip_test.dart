import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/item_stats.dart';
import 'package:goresave/features/editor/domain/item_tooltip.dart';
import 'package:goresave/features/editor/ui/item_stats_tooltip.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/providers/data_providers.dart';

import '../../../support/l10n_test_app.dart';

const _tooltip = ItemTooltip(
  title: 'Battle Sword',
  subtitle: 'One-Handed Sword',
  stats: [ItemTooltipRow('Edge Dmg', '33'), ItemTooltipRow('Value', '31')],
  protection: [
    ItemTooltipRow('Edge', '+90', iconName: 'T_Icon_Resistance_Edge'),
  ],
  protectionLabel: 'Protection',
  requirements: [
    ItemTooltipRow('Strength', '23', iconName: 'T_Icon_Strength'),
  ],
  requirementsLabel: 'Requirements:',
  description: 'Heavy, and it shows.',
);

void main() {
  testWidgets('the card lays the game\'s own sections out', (tester) async {
    await tester.pumpWidget(
      const ProviderScope(
        child: MaterialApp(
          home: Scaffold(
            body: Center(
              child: SizedBox(
                width: 320,
                child: ItemTooltipCard(tooltip: _tooltip),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    for (final text in const [
      'Battle Sword',
      'One-Handed Sword',
      'Edge Dmg',
      '33',
      'Protection',
      'Requirements:',
      'Strength',
      '23',
      'Heavy, and it shows.',
    ]) {
      expect(find.text(text), findsOneWidget, reason: '"$text" is missing');
    }

    // The name sits above the type, and both above the numbers — the reading
    // order the game's card has.
    double top(String text) => tester.getTopLeft(find.text(text)).dy;
    expect(top('Battle Sword'), lessThan(top('One-Handed Sword')));
    expect(top('One-Handed Sword'), lessThan(top('Edge Dmg')));
    expect(top('Edge Dmg'), lessThan(top('Requirements:')));
    expect(top('Requirements:'), lessThan(top('Heavy, and it shows.')));

    // Values are pushed to the right edge, away from their label.
    expect(
      tester.getTopRight(find.text('33')).dx,
      greaterThan(tester.getTopRight(find.text('Edge Dmg')).dx),
    );
  });

  testWidgets('the card is not a Material tooltip', (tester) async {
    await tester.pumpWidget(
      const ProviderScope(
        child: MaterialApp(
          home: Scaffold(body: ItemTooltipCard(tooltip: _tooltip)),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.byType(Tooltip), findsNothing);
  });

  group('on hover', () {
    Widget app({required ItemStatsCatalog stats}) => ProviderScope(
      overrides: [itemStatsCatalogProvider.overrideWith((ref) async => stats)],
      child: MaterialApp(
        localizationsDelegates: testLocalizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: const Scaffold(
          body: Center(
            child: ItemStatsTooltip(
              itemId: 'ItMw_1H_Sword_01',
              title: 'Battle Sword',
              child: SizedBox(width: 200, height: 40, child: Text('row')),
            ),
          ),
        ),
      ),
    );

    Future<TestGesture> hover(WidgetTester tester, Finder target) async {
      final gesture = await tester.createGesture(kind: PointerDeviceKind.mouse);
      await gesture.addPointer(location: Offset.zero);
      addTearDown(gesture.removePointer);
      await tester.pump();
      await gesture.moveTo(tester.getCenter(target));
      await tester.pumpAndSettle();
      return gesture;
    }

    testWidgets('shows the card, then takes it away again', (tester) async {
      final stats = ItemStatsCatalog.fromJsonString('''
{"schema": 1, "filters": [], "items": {
  "ItMw_1H_Sword_01": {"itemType": "Item_Weapon_Sword_OneHand", "value": 31,
    "damage": {"Item_Damage_Physical_Edge": 17},
    "requires": {"Strength": 14}}}}
''');
      await tester.pumpWidget(app(stats: stats));
      await tester.pumpAndSettle();
      expect(find.byType(ItemTooltipCard), findsNothing);

      // One frame after the pointer arrives, with no dwell delay in between.
      final gesture = await tester.createGesture(kind: PointerDeviceKind.mouse);
      await gesture.addPointer(location: Offset.zero);
      addTearDown(gesture.removePointer);
      await tester.pump();
      await gesture.moveTo(tester.getCenter(find.text('row')));
      await tester.pump();
      expect(
        find.byType(ItemTooltipCard),
        findsOneWidget,
        reason: 'the card must not wait out a hover delay',
      );
      await tester.pumpAndSettle();
      expect(find.text('Battle Sword'), findsOneWidget);
      // The item's own numbers, not a generic label.
      expect(find.text('17'), findsOneWidget);
      expect(find.text('14'), findsOneWidget);

      await gesture.moveTo(Offset.zero);
      await tester.pumpAndSettle();
      expect(find.byType(ItemTooltipCard), findsNothing);
    });

    testWidgets('an item the stats do not know keeps its plain row', (
      tester,
    ) async {
      await tester.pumpWidget(app(stats: const ItemStatsCatalog()));
      await tester.pumpAndSettle();

      await hover(tester, find.text('row'));
      expect(find.byType(ItemTooltipCard), findsNothing);
      expect(find.text('row'), findsOneWidget);
    });

    Color? rowTint(WidgetTester tester) {
      final container = tester.widget<AnimatedContainer>(
        find.ancestor(
          of: find.text('row'),
          matching: find.byType(AnimatedContainer),
        ),
      );
      return (container.decoration as BoxDecoration?)?.color;
    }

    testWidgets('the row under the pointer is tinted, and only while there', (
      tester,
    ) async {
      await tester.pumpWidget(app(stats: const ItemStatsCatalog()));
      await tester.pumpAndSettle();
      expect(rowTint(tester), Colors.transparent);

      final gesture = await hover(tester, find.text('row'));
      expect(rowTint(tester), isNot(Colors.transparent));
      expect(rowTint(tester)?.a, greaterThan(0));

      await gesture.moveTo(Offset.zero);
      await tester.pumpAndSettle();
      expect(rowTint(tester), Colors.transparent);
    });

    testWidgets('a row that already reacts on its own is left alone', (
      tester,
    ) async {
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            itemStatsCatalogProvider.overrideWith(
              (ref) async => const ItemStatsCatalog(),
            ),
          ],
          child: MaterialApp(
            localizationsDelegates: testLocalizationsDelegates,
            supportedLocales: AppLocalizations.supportedLocales,
            home: const Scaffold(
              body: Center(
                child: ItemStatsTooltip(
                  itemId: 'ItMw_1H_Sword_01',
                  title: 'Battle Sword',
                  highlightOnHover: false,
                  child: SizedBox(width: 200, height: 40, child: Text('row')),
                ),
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();

      await hover(tester, find.text('row'));
      expect(rowTint(tester), Colors.transparent);
    });
  });
}
