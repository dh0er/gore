import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_manager/app/domain/ui_settings.dart';
import 'package:gore_manager/core/core_service.dart';
import 'package:gore_manager/core/providers.dart';
import 'package:gore_manager/home_page.dart';
import 'package:gore_manager/l10n/app_localizations.dart';

/// A settings store that starts light and records every write so the test can
/// assert the theme-mode toggle both flips the provider and persists.
class _RecordingSettingsStore implements UiSettingsStore {
  UiSettings _current = const UiSettings();
  final List<ThemeMode> writtenThemeModes = [];

  @override
  UiSettings read() => _current;

  @override
  void write(UiSettings settings) {
    _current = settings;
    writtenThemeModes.add(settings.themeMode);
  }
}

Widget _app(FakeGoreCoreFfiService fake, _RecordingSettingsStore store) {
  return ProviderScope(
    overrides: [
      coreServiceProvider.overrideWithValue(fake),
      uiSettingsStoreProvider.overrideWithValue(store),
    ],
    // Consume themeModeProvider like the real app so flipping it actually
    // switches the active brightness (and thus the toggle icon).
    child: Consumer(
      builder: (context, ref, _) => MaterialApp(
        localizationsDelegates: const [
          AppLocalizations.delegate,
          GlobalMaterialLocalizations.delegate,
          GlobalWidgetsLocalizations.delegate,
          GlobalCupertinoLocalizations.delegate,
        ],
        supportedLocales: AppLocalizations.supportedLocales,
        theme: ThemeData.light(),
        // A dark theme must exist for themeMode: dark to have a visible effect.
        darkTheme: ThemeData.dark(),
        themeMode: ref.watch(themeModeProvider),
        home: const HomePage(),
      ),
    ),
  );
}

void main() {
  testWidgets('the app-bar theme toggle flips and persists themeModeProvider',
      (tester) async {
    final fake = FakeGoreCoreFfiService(
      responses: {
        'mgr_library_list': {'ok': true, 'mods': [], 'loadout': {'entries': []}},
        'mgr_analyze': {'ok': true, 'conflicts': []},
        'mgr_status': {
          'ok': true,
          'status': {'state': 'nothing_deployed'},
        },
      },
    );
    final store = _RecordingSettingsStore();
    await tester.pumpWidget(_app(fake, store));
    await tester.pumpAndSettle();

    final container = ProviderScope.containerOf(
      tester.element(find.byType(HomePage)),
    );
    // Default theme mode is light; the toggle shows the "go dark" icon.
    expect(container.read(themeModeProvider), ThemeMode.light);
    expect(find.byIcon(Icons.dark_mode), findsOneWidget);

    await tester.tap(find.byIcon(Icons.dark_mode));
    await tester.pumpAndSettle();

    // Provider flipped to dark and the change was written to the store.
    expect(container.read(themeModeProvider), ThemeMode.dark);
    expect(store.writtenThemeModes, contains(ThemeMode.dark));
    // The icon now offers the reverse action.
    expect(find.byIcon(Icons.light_mode), findsOneWidget);

    // Tapping again returns to light.
    await tester.tap(find.byIcon(Icons.light_mode));
    await tester.pumpAndSettle();
    expect(container.read(themeModeProvider), ThemeMode.light);
    expect(store.writtenThemeModes.last, ThemeMode.light);
  });

  testWidgets('the app-bar info button opens the about dialog', (tester) async {
    final fake = FakeGoreCoreFfiService(
      responses: {
        'mgr_library_list': {'ok': true, 'mods': [], 'loadout': {'entries': []}},
        'mgr_analyze': {'ok': true, 'conflicts': []},
        'mgr_status': {
          'ok': true,
          'status': {'state': 'nothing_deployed'},
        },
      },
    );
    final store = _RecordingSettingsStore();
    await tester.pumpWidget(_app(fake, store));
    await tester.pumpAndSettle();

    final l10n = await AppLocalizations.delegate.load(const Locale('en'));
    // Before tapping, only the app-bar title carries the product name.
    expect(find.text('GORE Mod Manager'), findsOneWidget);

    await tester.tap(find.widgetWithIcon(IconButton, Icons.info_outline));
    await tester.pumpAndSettle();

    // The dialog's copyright + license lines are unique to the about dialog,
    // and the product name now appears a second time (title + dialog).
    expect(find.text(l10n.aboutCopyright), findsOneWidget);
    expect(find.text(l10n.aboutLicense), findsOneWidget);
    expect(find.text('GORE Mod Manager'), findsNWidgets(2));
    expect(tester.takeException(), isNull);
  });
}
