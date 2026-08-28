import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/item_icon_catalog.dart';
import 'package:goresave/features/localization/ui/localization_settings.dart';
import 'package:goresave/l10n/app_localizations.dart';

void main() {
  testWidgets('game data card shows image status and manually reloads it', (
    tester,
  ) async {
    var imageLoads = 0;

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          itemIconCatalogProvider.overrideWith((ref) async {
            ref.watch(itemIconCatalogReloadProvider);
            imageLoads++;
            return const ItemIconCatalog(
              buildId: 'test',
              manifestPath: 'manifest.json',
              pathByItemId: {
                'apple': 'apple.png',
                'bread': 'bread.png',
                'water': 'water.png',
              },
            );
          }),
        ],
        child: const _TestApp(),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Game data'), findsOneWidget);
    expect(find.text('Game text'), findsOneWidget);
    expect(find.text('Item images'), findsOneWidget);
    expect(find.text('3 item images are ready.'), findsOneWidget);
    expect(imageLoads, 1);

    await tester.tap(find.text('Check / refresh item images'));
    await tester.pumpAndSettle();

    expect(imageLoads, 2);
    expect(tester.takeException(), isNull);
  });

  testWidgets('game data actions remain readable at compact width', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(520, 700);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          itemIconCatalogProvider.overrideWith(
            (ref) async => const ItemIconCatalog.empty(),
          ),
        ],
        child: const _TestApp(),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Extract / refresh localized text'), findsOneWidget);
    expect(find.text('Check / refresh item images'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });
}

class _TestApp extends StatelessWidget {
  const _TestApp();

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      locale: const Locale('en'),
      localizationsDelegates: const [
        AppLocalizations.delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
      ],
      supportedLocales: AppLocalizations.supportedLocales,
      home: const Scaffold(
        body: SingleChildScrollView(
          padding: EdgeInsets.all(20),
          child: GameDataSettingsCard(),
        ),
      ),
    );
  }
}
