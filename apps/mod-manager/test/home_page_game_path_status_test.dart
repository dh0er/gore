import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_manager/app/domain/ui_settings.dart';
import 'package:gore_manager/app/game_paths.dart';
import 'package:gore_manager/core/core_service.dart';
import 'package:gore_manager/core/providers.dart';
import 'package:gore_manager/home_page.dart';
import 'package:gore_manager/l10n/app_localizations.dart';

/// A settings store that starts with no game path so the startup status runs
/// against a null root (the set-path sentinel), letting the test isolate the
/// mgr_status call triggered by the *path change*.
class _EmptySettingsStore implements UiSettingsStore {
  UiSettings _current = const UiSettings();

  @override
  UiSettings read() => _current;

  @override
  void write(UiSettings settings) => _current = settings;
}

Widget _app(FakeGoreCoreFfiService fake, UiSettingsStore store) {
  return ProviderScope(
    overrides: [
      coreServiceProvider.overrideWithValue(fake),
      uiSettingsStoreProvider.overrideWithValue(store),
    ],
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
}

void main() {
  testWidgets('changing the game exe path refreshes status for the new root',
      (tester) async {
    final fake = FakeGoreCoreFfiService(
      responses: {
        'mgr_library_list': {
          'ok': true,
          'mods': [],
          'loadout': {'entries': []},
        },
        'mgr_analyze': {'ok': true, 'conflicts': []},
        'mgr_status': {
          'ok': true,
          'status': {'state': 'nothing_deployed'},
        },
      },
    );
    await tester.pumpWidget(_app(fake, _EmptySettingsStore()));
    await tester.pumpAndSettle();

    // Startup ran mgr_status with a null root -> the sentinel, no FFI call.
    expect(
      fake.calls.where((c) => c.command == 'mgr_status'),
      isEmpty,
    );

    // A path whose game root resolves purely by path shape (no real install).
    const exe = 'C:/games/gothic/G1R/Binaries/Win64/G1R-Win64-Shipping.exe';
    final expectedRoot = gameRootFromExe(exe);
    expect(expectedRoot, isNotNull);

    final container = ProviderScope.containerOf(
      tester.element(find.byType(HomePage)),
    );
    container.read(gameExePathProvider.notifier).set(exe);
    await tester.pumpAndSettle();

    // The path change triggered a status refresh with the new game root.
    final statusCall =
        fake.calls.firstWhere((c) => c.command == 'mgr_status');
    expect(statusCall.payload['game_root'], expectedRoot);
  });
}
