import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/catalog/domain/catalog_provider.dart';
import 'package:gore_mod/catalog/domain/item_entry.dart';
import 'package:gore_mod/catalog/ui/catalog_browser.dart';

void main() {
  final apple  = CatalogItem(id: 'ItFo_Apple',       displayName: 'Apple',  fields: []);
  final sword  = CatalogItem(id: 'ItMw_1H_Sword_01', displayName: 'Sword',  fields: []);
  final cheese = CatalogItem(id: 'ItFo_Cheese',      displayName: 'Cheese', fields: []);

  Widget buildBrowser({
    required List<CatalogItem> catalog,
    void Function(CatalogItem)? onSelected,
  }) {
    return ProviderScope(
      overrides: [
        catalogProvider.overrideWith((ref) async => catalog),
      ],
      child: MaterialApp(
        home: Scaffold(
          body: CatalogBrowser(
            onItemSelected: onSelected ?? (_) {},
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
}
