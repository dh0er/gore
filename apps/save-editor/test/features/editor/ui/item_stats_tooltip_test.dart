import 'dart:async';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/features/editor/domain/item_stats.dart';
import 'package:goresave/features/editor/domain/item_tooltip.dart';
import 'package:goresave/features/editor/ui/item_stats_tooltip.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/providers/data_providers.dart';
import 'package:goresave/ui/design/app_theme.dart';

import '../../../support/l10n_test_app.dart';

const _tooltip = ItemTooltip(
  title: 'Battle Sword',
  subtitle: 'One-Handed Sword',
  stats: [ItemTooltipRow('Edge Dmg', '33'), ItemTooltipRow('Value', '31')],
  protection: [
    ItemTooltipRow('Edge', '+90', iconName: 'T_Icon_Resistance_Edge'),
  ],
  protectionLabel: 'Protection',
  requirements: [ItemTooltipRow('Strength', '23', iconName: 'T_Icon_Strength')],
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

    testWidgets('the card appears when the stats land under the pointer', (
      tester,
    ) async {
      // Hovering while the catalog is still loading left no portal to show; it
      // only appeared once the row was left and entered again.
      final ready = Completer<ItemStatsCatalog>();
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            itemStatsCatalogProvider.overrideWith((ref) => ready.future),
          ],
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
        ),
      );
      await tester.pump();

      final gesture = await tester.createGesture(kind: PointerDeviceKind.mouse);
      await gesture.addPointer(location: Offset.zero);
      addTearDown(gesture.removePointer);
      await tester.pump();
      await gesture.moveTo(tester.getCenter(find.text('row')));
      await tester.pump();
      expect(find.byType(ItemTooltipCard), findsNothing);

      ready.complete(
        ItemStatsCatalog.fromJsonString('''
{"schema": 1, "filters": [], "items": {
  "ItMw_1H_Sword_01": {"itemType": "Item_Weapon_Sword_OneHand", "value": 31}}}
'''),
      );
      await tester.pumpAndSettle();
      expect(find.byType(ItemTooltipCard), findsOneWidget);
    });

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

  testWidgets('interface labels keep the interface face', (tester) async {
    const zh = Locale.fromSubtags(languageCode: 'zh', scriptCode: 'Hans');
    await tester.pumpWidget(
      MaterialApp(
        theme: buildGoresaveTheme(
          uiFontFamily: UiFontFamily.notoSerif,
          locale: const Locale('ja'),
          gameTextLocale: zh,
        ),
        locale: const Locale('ja'),
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: const Scaffold(
          body: ItemTooltipCard(
            gameTextLocale: zh,
            tooltip: ItemTooltip(
              title: '长剑',
              stats: [
                ItemTooltipRow('斬撃', '10'),
                ItemTooltipRow('価値', '31', catalogLabel: false),
                ItemTooltipRow('体力', '+1/秒 · 3秒', interfaceValueRun: '3秒'),
              ],
              recipe: [ItemTooltipRow('鉄', '')],
              recipeLabel: 'レシピ: 长剑',
              recipeProduct: '长剑',
              ingredientFor: [ItemTooltipRow('長剣', '')],
              ingredientForLabel: '材料',
            ),
          ),
        ),
      ),
    );

    String? familyOf(String text) {
      final rich = tester.widget<RichText>(
        find.byWidgetPredicate(
          (widget) => widget is RichText && widget.text.toPlainText() == text,
        ),
      );
      TextSpan? leaf;
      void walk(InlineSpan span) {
        if (span is! TextSpan) return;
        if (span.text == text) leaf = span;
        span.children?.forEach(walk);
      }

      walk(rich.text);
      return leaf?.style?.fontFamily;
    }

    expect(familyOf('长剑'), notoSerifScFontFamily);
    expect(familyOf('斬撃'), notoSerifScFontFamily);
    expect(familyOf('価値'), notoSerifJpFontFamily);
    expect(familyOf('材料'), notoSerifJpFontFamily);
    expect(familyOf('鉄'), notoSerifScFontFamily);
    expect(familyOf('長剣'), notoSerifScFontFamily);

    final recipe = tester.widget<RichText>(
      find.byWidgetPredicate(
        (widget) =>
            widget is RichText && widget.text.toPlainText() == 'レシピ: 长剑',
      ),
    );
    final runs = <TextSpan>[];
    void walkRecipe(InlineSpan span) {
      if (span is! TextSpan) return;
      if (span.text != null && span.text!.isNotEmpty) runs.add(span);
      span.children?.forEach(walkRecipe);
    }

    walkRecipe(recipe.text);
    expect(runs.map((span) => span.text), ['レシピ: ', '长剑']);
    expect(runs[0].style?.fontFamily, notoSerifJpFontFamily);
    expect(runs[1].style?.fontFamily, notoSerifScFontFamily);

    final duration = tester.widget<RichText>(
      find.byWidgetPredicate(
        (widget) =>
            widget is RichText && widget.text.toPlainText() == '+1/秒 · 3秒',
      ),
    );
    final durationRuns = <TextSpan>[];
    void walkDuration(InlineSpan span) {
      if (span is! TextSpan) return;
      if (span.text != null && span.text!.isNotEmpty) durationRuns.add(span);
      span.children?.forEach(walkDuration);
    }

    walkDuration(duration.text);
    expect(durationRuns.map((span) => span.text), ['+1/秒 · ', '3秒']);
    expect(durationRuns[0].style?.fontFamily, notoSerifScFontFamily);
    expect(durationRuns[1].style?.fontFamily, notoSerifJpFontFamily);
  });

  testWidgets('a fallback item title keeps the interface face', (tester) async {
    const zh = Locale.fromSubtags(languageCode: 'zh', scriptCode: 'Hans');
    await tester.pumpWidget(
      MaterialApp(
        theme: buildGoresaveTheme(
          uiFontFamily: UiFontFamily.notoSerif,
          locale: const Locale('ja'),
          gameTextLocale: zh,
        ),
        locale: const Locale('ja'),
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: const Scaffold(
          body: ItemTooltipCard(
            gameTextLocale: zh,
            tooltip: ItemTooltip(title: 'アイテム', titleFromCatalog: false),
          ),
        ),
      ),
    );

    final rich = tester.widget<RichText>(find.byType(RichText));
    final span = rich.text;
    expect(span, isA<TextSpan>());
    expect((span as TextSpan).style?.fontFamily, notoSerifJpFontFamily);
  });
}
