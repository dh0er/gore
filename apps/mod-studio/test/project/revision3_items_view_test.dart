import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/l10n/app_localizations.dart';
import 'package:gore_mod/project/revision3_item_catalog.dart';
import 'package:gore_mod/project/revision3_items_view.dart';

void main() {
  testWidgets('browses bundled item facts without exposing mutation actions', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1200, 760);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(_app());
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('revision3-items-wide-layout')),
      findsOneWidget,
    );
    expect(find.text('Apple'), findsWidgets);
    expect(find.text('ItFo_Apple'), findsWidgets);
    expect(find.text('Bundled reference'), findsOneWidget);
    expect(
      find.textContaining('not been refreshed or generation-qualified'),
      findsOneWidget,
    );
    expect(find.text('m_Value'), findsOneWidget);
    expect(find.text('int'), findsWidgets);
    expect(find.text('= 4'), findsOneWidget);
    expect(find.text('= 0'), findsOneWidget);
    expect(find.text('\u2265 0'), findsOneWidget);
    expect(find.text('\u2265 1'), findsNothing);
    expect(find.text('Edit'), findsNothing);
    expect(find.text('Create'), findsNothing);
    expect(find.text('Save'), findsNothing);
  });

  testWidgets('filters by search and category without inventing fields', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1200, 760);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(_app());
    await tester.pumpAndSettle();

    await tester.enterText(
      find.byKey(const Key('revision3-items-search')),
      'sword',
    );
    await tester.pump();
    expect(
      find.byKey(const Key('revision3-items-details-ItMw_Sword')),
      findsOneWidget,
    );
    expect(find.text('m_Weight'), findsOneWidget);

    await tester.tap(find.byKey(const Key('revision3-items-clear-search')));
    await tester.pump();
    await tester.tap(
      find.byKey(const Key('revision3-items-category-meleeWeapon')),
    );
    await tester.pump();
    expect(
      find.byKey(const Key('revision3-items-result-ItMw_Sword')),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-items-result-ItFo_Apple')),
      findsNothing,
    );

    await tester.enterText(
      find.byKey(const Key('revision3-items-search')),
      'unknown',
    );
    await tester.pump();
    expect(find.text('No items match'), findsOneWidget);

    await tester.tap(find.byKey(const Key('revision3-items-category-all')));
    await tester.pump();
    expect(
      find.byKey(const Key('revision3-items-details-ItMi_Unknown')),
      findsOneWidget,
    );
    expect(
      find.text('No modeled scalar fields are available for this item.'),
      findsOneWidget,
    );

    await tester.enterText(
      find.byKey(const Key('revision3-items-search')),
      'worldsplitter',
    );
    await tester.pump();
    expect(
      find.byKey(const Key('revision3-items-details-ItIg_Worldsplitter')),
      findsOneWidget,
    );
    expect(find.text('Special'), findsWidgets);
  });

  testWidgets('uses a compact drill-in and back pattern', (tester) async {
    tester.view.physicalSize = const Size(360, 640);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(_app());
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('revision3-items-browser')), findsOneWidget);
    expect(find.byKey(const Key('revision3-items-detail-name')), findsNothing);

    await tester.tap(
      find.byKey(const Key('revision3-items-result-ItFo_Apple')),
    );
    await tester.pump();
    expect(
      find.byKey(const Key('revision3-items-detail-name')),
      findsOneWidget,
    );
    expect(find.byKey(const Key('revision3-items-browser')), findsNothing);

    await tester.tap(find.byKey(const Key('revision3-items-back')));
    await tester.pump();
    expect(find.byKey(const Key('revision3-items-browser')), findsOneWidget);
  });

  testWidgets('does not overflow at 320x180 with 200 percent text', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(320, 180);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(_app(textScale: 2));
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('revision3-items-browser')), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('reports a load failure and retries the stable loader', (
    tester,
  ) async {
    var attempts = 0;
    Future<Revision3ItemCatalog> load() async {
      attempts++;
      if (attempts == 1) throw const FormatException('bad bundled catalog');
      return _catalog();
    }

    await tester.pumpWidget(_app(load: load));
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('revision3-items-load-error')), findsOneWidget);

    await tester.tap(find.byKey(const Key('revision3-items-retry')));
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('revision3-items-browser')), findsOneWidget);
    expect(attempts, 2);
  });

  testWidgets('load failure scrolls at 320x180 with 200 percent text', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(320, 180);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    Future<Revision3ItemCatalog> fail() async =>
        throw const FormatException('bad bundled catalog');

    await tester.pumpWidget(_app(textScale: 2, load: fail));
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('revision3-items-load-error-scroll')),
      findsOneWidget,
    );
    expect(find.byKey(const Key('revision3-items-retry')), findsOneWidget);
    expect(tester.takeException(), isNull);
  });
}

Widget _app({
  double textScale = 1,
  Future<Revision3ItemCatalog> Function()? load,
}) => MaterialApp(
  localizationsDelegates: const <LocalizationsDelegate<dynamic>>[
    AppLocalizations.delegate,
    GlobalMaterialLocalizations.delegate,
    GlobalWidgetsLocalizations.delegate,
    GlobalCupertinoLocalizations.delegate,
  ],
  supportedLocales: AppLocalizations.supportedLocales,
  builder: (context, child) => MediaQuery(
    data: MediaQuery.of(
      context,
    ).copyWith(textScaler: TextScaler.linear(textScale)),
    child: child!,
  ),
  home: Scaffold(body: Revision3ItemsView(load: load ?? _loadCatalog)),
);

Future<Revision3ItemCatalog> _loadCatalog() async => _catalog();

Revision3ItemCatalog _catalog() => Revision3ItemCatalog.fromJson(
  itemCatalogJson: '''
    [
      {"category":"melee_weapon","id":"ItMw_Sword"},
      {"category":"misc","id":"ItMi_Unknown"},
      {"category":"food","id":"ItFo_Apple"},
      {"category":"special","id":"ItIg_Worldsplitter"}
    ]
  ''',
  modelJson: '''
    {
      "classes": {
        "ItFo_Apple": {
          "fields": [
            {"name":"m_Value","type":"int","default":4,"min":0},
            {"name":"m_MaxStack","type":"int","default":0}
          ]
        },
        "ItMw_Sword": {
          "fields": [
            {"name":"m_Weight","type":"float","default":2.5}
          ]
        }
      }
    }
  ''',
);
