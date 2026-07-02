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
  Future<void> pumpHarness(WidgetTester tester) async {
    tester.view.physicalSize = const Size(1400, 900);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(ProviderScope(
      overrides: [
        locCatalogProvider.overrideWith((ref) => Future.value(catalog)),
      ],
      child: const MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(body: DialogeTab()),
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
}
