import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'app/domain/desktop_updater.dart';
import 'app/domain/ui_settings.dart';
import 'app/ui/ui_scale_root.dart';
import 'core/core_service.dart';
import 'core/providers.dart';
import 'core/ui/core_unavailable_page.dart';
import 'home_page.dart';
import 'l10n/app_localizations.dart';
import 'loc/game_lang.dart';
import 'ui/design/app_theme.dart';

class GoreManagerApp extends ConsumerWidget {
  const GoreManagerApp({super.key});
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final lang = gameLangByCode(ref.watch(localeProvider));
    final themeMode = ref.watch(themeModeProvider);
    final core = ref.watch(coreServiceProvider);
    final home = switch (coreBootstrapStateOf(core)) {
      CoreBootstrapReady() => const HomePage(),
      CoreBootstrapBlocked(:final failure) => CoreUnavailablePage(
        failure: failure,
      ),
    };
    return MaterialApp(
      title: 'gore-manager',
      // A background update check has no widget context of its own; this key
      // gives it one to show its dialog from.
      navigatorKey: updaterNavigatorKey,
      debugShowCheckedModeBanner: false,
      theme: buildGoreManagerTheme(),
      darkTheme: buildGoreManagerDarkTheme(),
      themeMode: themeMode,
      locale: lang.locale,
      localizationsDelegates: const [
        AppLocalizations.delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
      ],
      supportedLocales: AppLocalizations.supportedLocales,
      builder: (context, child) =>
          UiScaleRoot(child: child ?? const SizedBox.shrink()),
      home: home,
    );
  }
}
