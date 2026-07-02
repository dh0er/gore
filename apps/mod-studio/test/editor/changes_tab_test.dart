import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/catalog/domain/catalog_provider.dart';
import 'package:gore_mod/catalog/domain/item_entry.dart';
import 'package:gore_mod/editor/domain/override_entry.dart';
import 'package:gore_mod/editor/domain/overrides_notifier.dart';
import 'package:gore_mod/editor/ui/changes_tab.dart';
import 'package:gore_mod/l10n/app_localizations.dart';
import 'package:gore_mod/loc/domain/loc_catalog_provider.dart';
import 'package:gore_mod/loc/domain/loc_edits_notifier.dart';
import 'package:gore_mod/textures/domain/texture_replacements_notifier.dart';

void main() {
  const apple500 = OverrideEntry(
    classId: 'ItFo_Apple', field: 'm_Value', oldValue: 4, newValue: 500,
  );
  final apple = CatalogItem(id: 'ItFo_Apple',       displayName: 'Apple', fields: []);
  final sword = CatalogItem(id: 'ItMw_1H_Sword_01', displayName: 'Sword', fields: []);
  const locCatalog = <String, Map<String, String>>{
    'info_aaron_001': {'de_A': 'Hallo', 'en_A': 'Hello'},
    'info_bob_001': {'de_A': 'Moin', 'en_A': 'Hi'},
    // A non-dialog loc id (item name): never part of the Dialogs section.
    'itfo_apple_name': {'de_A': 'Apfel', 'en_A': 'Apple'},
  };
  const stagedTexture = TextureReplacement(
    asset: 'Game/Textures/T_Apple_D',
    imagePath: r'C:\mods\apple.png',
  );

  /// Container with one staged item override, one loc edit and one texture
  /// replacement (all pure notifiers — no FFI), plus fake catalogs for the
  /// embedded Items/Dialoge views.
  ProviderContainer makeContainer() {
    final container = ProviderContainer(overrides: [
      catalogProvider.overrideWith((ref) async => [apple, sword]),
      locCatalogProvider.overrideWith((ref) => Future.value(locCatalog)),
    ]);
    container.read(overridesProvider.notifier).setOverride(apple500);
    container
        .read(locEditsProvider.notifier)
        .setEdit('info_aaron_001', 'de_A', 'Servus');
    container
        .read(textureReplacementsProvider.notifier)
        .setReplacement(stagedTexture);
    return container;
  }

  // The embedded Items/Dialoge views reserve a 560px browser next to the
  // 230px Changes sidebar; the 800x600 test default would overflow their
  // editor panes. Use a desktop-like window, matching the real app.
  Future<void> pumpHarness(WidgetTester tester, ProviderContainer container) async {
    tester.view.physicalSize = const Size(1800, 900);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(UncontrolledProviderScope(
      container: container,
      child: MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: const Scaffold(body: ChangesTab()),
      ),
    ));
    await tester.pumpAndSettle();
  }

  testWidgets('sidebar shows per-domain counts and defaults to the All list',
      (tester) async {
    final container = makeContainer();
    addTearDown(container.dispose);
    await pumpHarness(tester, container);

    // One override + one loc edit + one texture replacement = 3 total.
    expect(find.text('All (3)'), findsOneWidget);
    expect(find.text('Items (1)'), findsOneWidget);
    expect(find.text('Dialogs (1)'), findsOneWidget);
    expect(find.text('Audio (0)'), findsOneWidget);
    expect(find.text('Textures (1)'), findsOneWidget);
    expect(find.text('Scripts (0)'), findsOneWidget);

    // Default section is "All": the flat OverridesPanel with its own header.
    expect(find.text('Changes (3)'), findsOneWidget);
    expect(find.text('ItFo_Apple.m_Value'), findsOneWidget);
  });

  testWidgets('Dialogs section shows the dialog browser filtered to edited ids',
      (tester) async {
    final container = makeContainer();
    addTearDown(container.dispose);
    await pumpHarness(tester, container);

    await tester.tap(find.text('Dialogs (1)'));
    await tester.pumpAndSettle();

    // Only the edited line's group/line survive the filter.
    expect(find.text('Aaron (1)'), findsOneWidget);
    expect(find.text('info_aaron_001'), findsOneWidget);
    expect(find.text('Bob (1)'), findsNothing);
    expect(find.text('info_bob_001'), findsNothing);
    // The All list is gone.
    expect(find.text('Changes (3)'), findsNothing);
  });

  testWidgets('Items section shows the items view filtered to changed ids',
      (tester) async {
    final container = makeContainer();
    addTearDown(container.dispose);
    await pumpHarness(tester, container);

    await tester.tap(find.text('Items (1)'));
    await tester.pumpAndSettle();

    // Item browser renders restricted to the overridden item.
    expect(find.text('Search items'), findsOneWidget);
    expect(find.text('Apple'), findsOneWidget);
    expect(find.text('Sword'), findsNothing);
    expect(find.text('Select an item to edit its fields.'), findsOneWidget);
  });

  testWidgets('non-dialog loc edits count toward All but not Dialogs',
      (tester) async {
    final container = makeContainer();
    addTearDown(container.dispose);
    // Stage an item-NAME loc edit (non-dialog prefix) on top of the fixture.
    container
        .read(locEditsProvider.notifier)
        .setEdit('itfo_apple_name', 'de_A', 'Superapfel');
    await pumpHarness(tester, container);

    // entryCount (All / OverridesPanel header) includes it, Dialogs does not.
    expect(find.text('All (4)'), findsOneWidget);
    expect(find.text('Changes (4)'), findsOneWidget);
    expect(find.text('Dialogs (1)'), findsOneWidget);

    // The filtered dialog view doesn't surface it either.
    await tester.tap(find.text('Dialogs (1)'));
    await tester.pumpAndSettle();
    expect(find.text('Aaron (1)'), findsOneWidget);
    expect(find.text('itfo_apple_name'), findsNothing);
  });

  testWidgets('a dialog id edited in two languages counts once in Dialogs',
      (tester) async {
    final container = makeContainer();
    addTearDown(container.dispose);
    // Second language for the already-staged info_aaron_001 edit.
    container
        .read(locEditsProvider.notifier)
        .setEdit('info_aaron_001', 'en_A', 'Howdy');
    await pumpHarness(tester, container);

    // All counts id x language pairs (2), Dialogs counts distinct ids (1).
    expect(find.text('All (4)'), findsOneWidget);
    expect(find.text('Dialogs (1)'), findsOneWidget);

    await tester.tap(find.text('Dialogs (1)'));
    await tester.pumpAndSettle();
    expect(find.text('Aaron (1)'), findsOneWidget);
    expect(find.text('info_aaron_001'), findsOneWidget);
  });

  testWidgets('counts update live after un-staging through the notifiers',
      (tester) async {
    final container = makeContainer();
    addTearDown(container.dispose);
    await pumpHarness(tester, container);

    container.read(locEditsProvider.notifier).removeEdit('info_aaron_001', 'de_A');
    await tester.pumpAndSettle();
    expect(find.text('All (2)'), findsOneWidget);
    expect(find.text('Dialogs (0)'), findsOneWidget);

    container.read(textureReplacementsProvider.notifier).clearAll();
    await tester.pumpAndSettle();
    expect(find.text('All (1)'), findsOneWidget);
    expect(find.text('Textures (0)'), findsOneWidget);
    expect(find.text('Changes (1)'), findsOneWidget);
  });
}
