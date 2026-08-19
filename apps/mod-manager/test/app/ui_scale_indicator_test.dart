import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_manager/app/domain/ui_settings.dart';
import 'package:gore_manager/core/core_service.dart';
import 'package:gore_manager/core/providers.dart';
import 'package:gore_manager/home_page.dart';
import 'package:gore_manager/l10n/app_localizations.dart';

FakeGoreCoreFfiService _core() => FakeGoreCoreFfiService(
  responses: {
    'mgr_library_list': {
      'ok': true,
      'mods': <Object?>[],
      'loadout': {'format': 1, 'entries': <Object?>[]},
    },
    'mgr_analyze': {'ok': true, 'conflicts': <Object?>[]},
  },
);

Widget _app() => ProviderScope(
  overrides: [coreServiceProvider.overrideWithValue(_core())],
  child: MaterialApp(
    localizationsDelegates: const [
      AppLocalizations.delegate,
      GlobalMaterialLocalizations.delegate,
      GlobalWidgetsLocalizations.delegate,
      GlobalCupertinoLocalizations.delegate,
    ],
    supportedLocales: AppLocalizations.supportedLocales,
    home: const HomePage(),
  ),
);

void _window(WidgetTester tester, Size size, {double textScale = 1}) {
  tester.view.physicalSize = size;
  tester.view.devicePixelRatio = 1;
  tester.platformDispatcher.textScaleFactorTestValue = textScale;
  addTearDown(tester.view.resetPhysicalSize);
  addTearDown(tester.view.resetDevicePixelRatio);
  addTearDown(tester.platformDispatcher.clearTextScaleFactorTestValue);
}

void main() {
  testWidgets('the title bar shows the current UI scale and follows it', (
    tester,
  ) async {
    _window(tester, const Size(1280, 800));
    await tester.pumpWidget(_app());
    await tester.pumpAndSettle();

    final indicator = find.byKey(const ValueKey('ui-scale-indicator'));
    expect(indicator, findsOneWidget);
    expect(
      find.descendant(of: indicator, matching: find.text('100%')),
      findsOneWidget,
    );

    final container = ProviderScope.containerOf(
      tester.element(find.byType(HomePage)),
    );
    container.read(uiScaleProvider.notifier).set(1.25);
    await tester.pumpAndSettle();

    expect(
      find.descendant(of: indicator, matching: find.text('125%')),
      findsOneWidget,
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('a narrow title bar drops the indicator instead of overflowing', (
    tester,
  ) async {
    _window(tester, const Size(700, 460), textScale: 2);
    await tester.pumpWidget(_app());
    await tester.pumpAndSettle();

    // The window buttons must survive; the same percentage stays readable in
    // Settings, so hiding the chip loses nothing.
    expect(find.byKey(const ValueKey('ui-scale-indicator')), findsNothing);
    expect(tester.takeException(), isNull);
  });
}
