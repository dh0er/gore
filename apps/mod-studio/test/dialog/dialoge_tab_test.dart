import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/dialog/ui/dialoge_tab.dart';
import 'package:gore_mod/l10n/app_localizations.dart';
import 'package:gore_mod/loc/domain/loc_catalog_provider.dart';

const _kPlaceholder = 'Select a dialog line to edit';

void main() {
  const catalog = <String, Map<String, String>>{
    'info_aaron_001': {'de_A': 'Hallo', 'en_A': 'Hello'},
    'info_bob_001': {'de_A': 'Moin', 'en_A': 'Hi'},
  };

  // Desktop-sized surface: the tab's fixed 560px browser leaves the editor
  // pane too narrow on the 800x600 test default, overflowing its header row.
  Future<void> pumpHarness(WidgetTester tester, {Set<String>? onlyIds}) async {
    tester.view.physicalSize = const Size(1400, 900);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(ProviderScope(
      overrides: [
        locCatalogProvider.overrideWith((ref) => Future.value(catalog)),
      ],
      child: MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(body: DialogeTab(onlyIds: onlyIds)),
      ),
    ));
    await tester.pumpAndSettle();
  }

  testWidgets('switching speaker groups clears a selection from the other group',
      (tester) async {
    await pumpHarness(tester);

    // Group A (Aaron) is auto-selected; pick its line.
    await tester.tap(find.text('info_aaron_001'));
    await tester.pumpAndSettle();
    expect(find.text(_kPlaceholder), findsNothing);
    // Selected id shows in both the line list and the editor header.
    expect(find.text('info_aaron_001'), findsNWidgets(2));

    // Tap group B in the sidebar: the editor must not keep showing a line
    // from group A.
    await tester.tap(find.text('Bob (1)'));
    await tester.pumpAndSettle();
    expect(find.text(_kPlaceholder), findsOneWidget);
    expect(find.text('info_bob_001'), findsOneWidget);
    expect(find.text('info_aaron_001'), findsNothing);

    // Back to group A: the cleared selection must not reappear.
    await tester.tap(find.text('Aaron (1)'));
    await tester.pumpAndSettle();
    expect(find.text(_kPlaceholder), findsOneWidget);
    expect(find.text('info_aaron_001'), findsOneWidget);
  });

  testWidgets('re-tapping the selected line\'s own group keeps the selection',
      (tester) async {
    await pumpHarness(tester);

    await tester.tap(find.text('info_aaron_001'));
    await tester.pumpAndSettle();
    expect(find.text('info_aaron_001'), findsNWidgets(2));

    await tester.tap(find.text('Aaron (1)'));
    await tester.pumpAndSettle();
    expect(find.text(_kPlaceholder), findsNothing);
    expect(find.text('info_aaron_001'), findsNWidgets(2));
  });

  testWidgets('onlyIds restricts sidebar groups and lines to the given ids',
      (tester) async {
    await pumpHarness(tester, onlyIds: {'info_bob_001'});

    // Only Bob's group survives the filter; Aaron is gone entirely.
    expect(find.text('Bob (1)'), findsOneWidget);
    expect(find.text('Aaron (1)'), findsNothing);
    expect(find.text('info_bob_001'), findsOneWidget);
    expect(find.text('info_aaron_001'), findsNothing);
  });

  testWidgets('search inside a filtered view only searches filtered lines',
      (tester) async {
    await pumpHarness(tester, onlyIds: {'info_bob_001'});

    // 'aaron' matches a catalog line, but that line is filtered out.
    await tester.enterText(find.byType(TextField).first, 'aaron');
    await tester.pumpAndSettle();
    expect(find.text('info_aaron_001'), findsNothing);
    expect(find.text('No dialog lines match'), findsWidgets);

    // The filtered line itself is still searchable (by text value).
    await tester.enterText(find.byType(TextField).first, 'moin');
    await tester.pumpAndSettle();
    expect(find.text('info_bob_001'), findsOneWidget);
  });

  testWidgets('empty onlyIds set shows the no-match hint', (tester) async {
    await pumpHarness(tester, onlyIds: const {});

    expect(find.text('No dialog lines match'), findsOneWidget);
    expect(find.text(_kPlaceholder), findsOneWidget);
    expect(find.text('info_aaron_001'), findsNothing);
    expect(find.text('info_bob_001'), findsNothing);
  });

  testWidgets(
      'filtered editor ignores an out-of-filter shared selection '
      'without clearing it', (tester) async {
    tester.view.physicalSize = const Size(1400, 900);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);

    final container = ProviderContainer(overrides: [
      locCatalogProvider.overrideWith((ref) => Future.value(catalog)),
    ]);
    addTearDown(container.dispose);

    Future<void> pumpFiltered(Set<String> onlyIds) async {
      await tester.pumpWidget(UncontrolledProviderScope(
        container: container,
        child: MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: Scaffold(body: DialogeTab(onlyIds: onlyIds)),
        ),
      ));
      await tester.pumpAndSettle();
    }

    // Selection made outside the filter (e.g. on the main Dialoge tab).
    container.read(selectedDialogIdProvider.notifier).state =
        'info_aaron_001';
    await pumpFiltered({'info_bob_001'});

    // The filtered editor shows the placeholder, not Aaron's line...
    expect(find.text(_kPlaceholder), findsOneWidget);
    expect(find.text('info_aaron_001'), findsNothing);
    // ...and the shared selection is NOT cleared (the main tab owns it).
    expect(container.read(selectedDialogIdProvider), 'info_aaron_001');

    // Selecting a line inside the filter still opens the editor.
    await tester.tap(find.text('info_bob_001'));
    await tester.pumpAndSettle();
    expect(find.text(_kPlaceholder), findsNothing);
    expect(find.text('info_bob_001'), findsNWidgets(2));
    expect(container.read(selectedDialogIdProvider), 'info_bob_001');

    // Filter shrinks past the selected line (last staged edit removed):
    // back to the placeholder, shared selection again untouched.
    await pumpFiltered(const {});
    expect(find.text(_kPlaceholder), findsOneWidget);
    expect(find.text('info_bob_001'), findsNothing);
    expect(container.read(selectedDialogIdProvider), 'info_bob_001');
  });

  testWidgets('unfiltered editor still follows the shared selection',
      (tester) async {
    tester.view.physicalSize = const Size(1400, 900);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);

    final container = ProviderContainer(overrides: [
      locCatalogProvider.overrideWith((ref) => Future.value(catalog)),
    ]);
    addTearDown(container.dispose);
    container.read(selectedDialogIdProvider.notifier).state =
        'info_aaron_001';

    await tester.pumpWidget(UncontrolledProviderScope(
      container: container,
      child: MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: const Scaffold(body: DialogeTab()),
      ),
    ));
    await tester.pumpAndSettle();

    // No filter: the editor renders the shared selection (list + header).
    expect(find.text(_kPlaceholder), findsNothing);
    expect(find.text('info_aaron_001'), findsNWidgets(2));
  });
}
