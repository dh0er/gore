import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/catalog/domain/catalog_provider.dart';
import 'package:gore_mod/catalog/domain/item_entry.dart';
import 'package:gore_mod/catalog/ui/catalog_browser.dart';
import 'package:gore_mod/l10n/app_localizations.dart';

void main() {
  final apple  = CatalogItem(id: 'ItFo_Apple',       displayName: 'Apple',  fields: []);
  final sword  = CatalogItem(id: 'ItMw_1H_Sword_01', displayName: 'Sword',  fields: []);
  final cheese = CatalogItem(id: 'ItFo_Cheese',      displayName: 'Cheese', fields: []);

  Widget buildBrowser({
    required List<CatalogItem> catalog,
    void Function(CatalogItem)? onSelected,
    Set<String>? onlyIds,
  }) {
    return ProviderScope(
      overrides: [
        catalogProvider.overrideWith((ref) async => catalog),
      ],
      child: MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(
          body: CatalogBrowser(
            onItemSelected: onSelected ?? (_) {},
            onlyIds: onlyIds,
          ),
        ),
      ),
    );
  }

  testWidgets('shows items in catalog when loaded', (tester) async {
    await tester.pumpWidget(buildBrowser(catalog: [apple, sword]));
    await tester.pumpAndSettle();
    // Default category is the first group (meleeWeapon before food)
    expect(find.text('Sword'), findsOneWidget);
  });

  testWidgets('search filters to matching items across categories', (tester) async {
    await tester.pumpWidget(buildBrowser(catalog: [apple, cheese, sword]));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField), 'apple');
    await tester.pumpAndSettle();
    expect(find.text('Apple'),  findsOneWidget);
    expect(find.text('Cheese'), findsNothing);
    expect(find.text('Sword'),  findsNothing);
  });

  testWidgets('tapping an item calls onItemSelected', (tester) async {
    CatalogItem? tapped;
    await tester.pumpWidget(buildBrowser(
      catalog: [apple, sword],
      onSelected: (item) => tapped = item,
    ));
    await tester.pumpAndSettle();
    // Navigate to food category
    await tester.tap(find.text('Food & potions (1)'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Apple'));
    expect(tapped?.id, 'ItFo_Apple');
  });

  testWidgets('shows category sidebar tiles', (tester) async {
    await tester.pumpWidget(buildBrowser(catalog: [apple, sword]));
    await tester.pumpAndSettle();
    expect(find.text('Melee weapons (1)'), findsOneWidget);
    expect(find.text('Food & potions (1)'), findsOneWidget);
  });

  testWidgets('shows No items match when search has no results', (tester) async {
    await tester.pumpWidget(buildBrowser(catalog: [apple]));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField), 'xyzzy');
    await tester.pumpAndSettle();
    expect(find.text('No items match'), findsOneWidget);
  });

  testWidgets('onlyIds restricts items and drops empty categories', (tester) async {
    await tester.pumpWidget(buildBrowser(
      catalog: [apple, cheese, sword],
      onlyIds: {'ItFo_Apple'},
    ));
    await tester.pumpAndSettle();
    // Only the food category survives (melee weapons has no remaining items).
    expect(find.text('Food & potions (1)'), findsOneWidget);
    expect(find.textContaining('Melee weapons'), findsNothing);
    expect(find.text('Apple'),  findsOneWidget);
    expect(find.text('Cheese'), findsNothing);
    expect(find.text('Sword'),  findsNothing);
  });

  testWidgets('onlyIds applies before search', (tester) async {
    await tester.pumpWidget(buildBrowser(
      catalog: [apple, cheese, sword],
      onlyIds: {'ItFo_Apple', 'ItFo_Cheese'},
    ));
    await tester.pumpAndSettle();
    // An excluded item is not reachable via search.
    await tester.enterText(find.byType(TextField), 'sword');
    await tester.pumpAndSettle();
    expect(find.text('No items match'), findsOneWidget);
    // An included item still is.
    await tester.enterText(find.byType(TextField), 'cheese');
    await tester.pumpAndSettle();
    expect(find.text('Cheese'), findsOneWidget);
  });

  testWidgets('empty onlyIds shows the generic empty state without crashing',
      (tester) async {
    await tester.pumpWidget(buildBrowser(
      catalog: [apple, cheese, sword],
      onlyIds: const {},
    ));
    await tester.pumpAndSettle();
    expect(find.text('No items match'), findsOneWidget);
    expect(find.byType(ListTile), findsNothing);
    expect(tester.takeException(), isNull);
  });
}
