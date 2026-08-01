import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/l10n/app_localizations.dart';

/// The standard localization delegates used by the real app, exposed for tests
/// that pump a widget directly (without [GoresaveApp]). Without these, any
/// widget that calls `AppLocalizations.of(context)` throws.
const List<LocalizationsDelegate<dynamic>> testLocalizationsDelegates = [
  AppLocalizations.delegate,
  GlobalMaterialLocalizations.delegate,
  GlobalWidgetsLocalizations.delegate,
  GlobalCupertinoLocalizations.delegate,
];

/// Wraps [home] in a [ProviderScope] + [MaterialApp] that supplies the app's
/// localization delegates (default locale: English), so localized widgets
/// render with their English values in tests.
///
/// Pass [locale] to pump the same widget in another language — the only way to
/// catch a string that renders English inside a translated screen.
Widget wrapWithL10n(Widget home, {Locale locale = const Locale('en')}) {
  return ProviderScope(
    child: MaterialApp(
      locale: locale,
      localizationsDelegates: testLocalizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      home: home,
    ),
  );
}
