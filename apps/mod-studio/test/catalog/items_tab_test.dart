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
    // The unfiltered main tab shows the full field set.
    expect(
      tester.widget<FieldEditor>(find.byType(FieldEditor)).onlyEdited,
      isFalse,
    );
  });

  testWidgets('filtered view puts the field editor in edited-only mode',
      (tester) async {
    useDesktopSurface(tester);
    await tester.pumpWidget(buildTab(
      catalog: [apple, sword],
      onlyIds: {'ItFo_Apple'},
    ));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Apple'));
    await tester.pumpAndSettle();
    expect(
      tester.widget<FieldEditor>(find.byType(FieldEditor)).onlyEdited,
      isTrue,
    );
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

  testWidgets('filtered view ignores an out-of-filter shared selection',
      (tester) async {
    useDesktopSurface(tester);
    await tester.pumpWidget(buildTab(
      catalog: [apple, sword],
      onlyIds: {'ItFo_Apple'},
    ));
    await tester.pumpAndSettle();
    final container = ProviderScope.containerOf(
      tester.element(find.byType(ItemsTab)),
      listen: false,
    );
    // A selection made elsewhere (e.g. the main Items tab) on an item that is
    // not part of this filter: the browser doesn't list it, so no editor.
    container.read(selectedItemProvider.notifier).state = sword;
    await tester.pumpAndSettle();
    expect(find.byType(FieldEditor), findsNothing);
    expect(find.text('Select an item to edit its fields.'), findsOneWidget);
    // The guard is view-level only — the shared selection stays untouched.
    expect(container.read(selectedItemProvider)?.id, 'ItMw_1H_Sword_01');
    // Selecting inside the filtered browser still works.
    await tester.tap(find.text('Apple'));
    await tester.pumpAndSettle();
    expect(find.byType(FieldEditor), findsOneWidget);
    expect(find.text('Select an item to edit its fields.'), findsNothing);
    expect(container.read(selectedItemProvider)?.id, 'ItFo_Apple');
  });
}
