import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'app/domain/ui_settings.dart';
import 'app/ui/ui_scale_root.dart';
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
    return MaterialApp(
      title: 'gore-manager',
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
      home: const HomePage(),
    );
  }
}
