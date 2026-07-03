import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'app/domain/ui_settings.dart';
import 'home_page.dart';
import 'l10n/app_localizations.dart';
import 'loc/game_lang.dart';

class GoreManagerApp extends ConsumerWidget {
  const GoreManagerApp({super.key});
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final lang = gameLangByCode(ref.watch(localeProvider));
    return MaterialApp(
      title: 'gore-manager',
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
