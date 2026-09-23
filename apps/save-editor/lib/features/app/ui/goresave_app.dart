import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/features/app/ui/ui_scale_root.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/loc/game_lang.dart';
import 'package:goresave/providers/data_providers.dart';
import 'package:goresave/ui/design/app_theme.dart';

class GoresaveApp extends ConsumerWidget {
  const GoresaveApp({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final router = ref.watch(routerProvider).router;
    final themeMode = ref.watch(themeModeProvider);
    final selectedUiFont = ref.watch(uiFontFamilyProvider);
    final uiLang = uiLangByCode(ref.watch(localeProvider));
    final gameLang = gameLangByCode(ref.watch(gameTextLocaleProvider));
    final uiFont = effectiveUiFontFamily(selectedUiFont, uiLang.locale);
    return MaterialApp.router(
      // Window/OS title is language-independent on purpose — always the
      // product name, never the localized UI string.
      title: 'GORE Save Editor',
      debugShowCheckedModeBanner: false,
      theme: buildGoresaveTheme(
        uiFontFamily: uiFont,
        locale: uiLang.locale,
        gameTextLocale: gameLang.locale,
      ),
      darkTheme: buildGoresaveDarkTheme(
        uiFontFamily: uiFont,
        locale: uiLang.locale,
        gameTextLocale: gameLang.locale,
      ),
      themeMode: themeMode,
      locale: uiLang.locale,
      localizationsDelegates: const [
        AppLocalizations.delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
      ],
      supportedLocales: AppLocalizations.supportedLocales,
      builder: (context, child) => GameTextScript(
        locale: gameLang.locale,
        child: UiScaleRoot(child: child ?? const SizedBox.shrink()),
      ),
      routerConfig: router,
    );
  }
}
