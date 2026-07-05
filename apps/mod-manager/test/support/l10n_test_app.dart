import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gore_manager/l10n/app_localizations.dart';

/// The standard localization delegates used by the real app, exposed for tests
/// that pump a widget directly (without [GoreManagerApp]). Without these, any
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
Widget wrapWithL10n(Widget home) {
  return ProviderScope(
    child: MaterialApp(
      locale: const Locale('en'),
      localizationsDelegates: testLocalizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      home: home,
    ),
  );
}
