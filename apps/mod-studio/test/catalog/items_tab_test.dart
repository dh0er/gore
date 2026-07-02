import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/catalog/domain/catalog_provider.dart';
import 'package:gore_mod/catalog/domain/item_entry.dart';
import 'package:gore_mod/catalog/ui/catalog_browser.dart';
import 'package:gore_mod/catalog/ui/items_tab.dart';
import 'package:gore_mod/editor/ui/field_editor.dart';
import 'package:gore_mod/l10n/app_localizations.dart';

void main() {
  final apple = CatalogItem(id: 'ItFo_Apple',       displayName: 'Apple', fields: []);
  final sword = CatalogItem(id: 'ItMw_1H_Sword_01', displayName: 'Sword', fields: []);

  // The tab reserves 560px for the browser; the default 800x600 test surface
  // leaves too little room for the field editor header. Use a desktop-like
  // window, matching the real app.
  void useDesktopSurface(WidgetTester tester) {
    tester.view.physicalSize = const Size(1600, 900);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);
  }

  Widget buildTab({
    required List<CatalogItem> catalog,
    Set<String>? onlyIds,
  }) {
    return ProviderScope(
      overrides: [
        catalogProvider.overrideWith((ref) async => catalog),
      ],
      child: MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(body: ItemsTab(onlyIds: onlyIds)),
      ),
    );
  }

  testWidgets('renders browser and placeholder when nothing is selected',
      (tester) async {
    useDesktopSurface(tester);
    await tester.pumpWidget(buildTab(catalog: [apple, sword]));
    await tester.pumpAndSettle();
    expect(find.byType(CatalogBrowser), findsOneWidget);
    expect(find.text('Select an item to edit its fields.'), findsOneWidget);
    expect(find.byType(FieldEditor), findsNothing);
  });

  testWidgets('selecting an item shows the field editor', (tester) async {
    useDesktopSurface(tester);
    await tester.pumpWidget(buildTab(catalog: [apple, sword]));
    await tester.pumpAndSettle();
    // Default category is melee weapons; select the sword.
    await tester.tap(find.text('Sword'));
    await tester.pumpAndSettle();
    expect(find.byType(FieldEditor), findsOneWidget);
    expect(find.text('Select an item to edit its fields.'), findsNothing);
  });

  testWidgets('onlyIds is forwarded to the catalog browser', (tester) async {
    useDesktopSurface(tester);
    await tester.pumpWidget(buildTab(
      catalog: [apple, sword],
      onlyIds: {'ItFo_Apple'},
    ));
    await tester.pumpAndSettle();
    expect(find.text('Apple'), findsOneWidget);
    expect(find.text('Sword'), findsNothing);
    expect(find.textContaining('Melee weapons'), findsNothing);
  });

  testWidgets('empty onlyIds renders the generic empty state', (tester) async {
    useDesktopSurface(tester);
    await tester.pumpWidget(buildTab(
      catalog: [apple, sword],
      onlyIds: const {},
    ));
    await tester.pumpAndSettle();
    expect(find.text('No items match'), findsOneWidget);
    expect(find.text('Select an item to edit its fields.'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });
}
