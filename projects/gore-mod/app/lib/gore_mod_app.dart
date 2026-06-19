import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'app/domain/ui_settings.dart';
import 'app/ui/app_theme.dart';
import 'home_page.dart';
import 'l10n/app_localizations.dart';
import 'loc/game_lang.dart';

class GoreModApp extends ConsumerWidget {
  const GoreModApp({super.key});
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final themeMode = ref.watch(themeModeProvider);
    final lang = gameLangByCode(ref.watch(localeProvider));
    return MaterialApp(
      title: 'gore-mod',
      themeMode: themeMode,
      theme: buildGoreModTheme(),
      darkTheme: buildGoreModDarkTheme(),
      locale: lang.locale,
      localizationsDelegates: const [
        AppLocalizations.delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
      ],
      supportedLocales: AppLocalizations.supportedLocales,
      home: const HomePage(),
    );
  }
}
