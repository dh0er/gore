import 'package:flutter/material.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';

const podkovaFontFamily = 'Podkova';
const notoSerifFontFamily = 'NotoSerif';
const notoSerifJpFontFamily = 'NotoSerifJP';
const notoSerifScFontFamily = 'NotoSerifSC';

/// Keeps technical values monospaced for the system font, while honoring a
/// bundled font when the user applies it to the entire interface.
String uiAwareMonospaceFontFamily(
  BuildContext context, {
  String fallback = 'Consolas',
}) {
  final activeFamily = Theme.of(context).textTheme.bodyMedium?.fontFamily;
  return switch (activeFamily) {
    podkovaFontFamily ||
    notoSerifFontFamily ||
    notoSerifJpFontFamily ||
    notoSerifScFontFamily => activeFamily!,
    _ => fallback,
  };
}

String uiFontFamilyName(UiFontFamily font, Locale locale) => switch (font) {
  UiFontFamily.system => 'Segoe UI',
  UiFontFamily.podkova => podkovaFontFamily,
  UiFontFamily.notoSerif when locale.languageCode == 'ja' =>
    notoSerifJpFontFamily,
  UiFontFamily.notoSerif when locale.languageCode == 'zh' =>
    notoSerifScFontFamily,
  UiFontFamily.notoSerif => notoSerifFontFamily,
};

ThemeData buildGoresaveTheme({
  UiFontFamily uiFontFamily = UiFontFamily.system,
  Locale locale = const Locale('en'),
}) => _buildTheme(Brightness.light, uiFontFamily: uiFontFamily, locale: locale);

ThemeData buildGoresaveDarkTheme({
  UiFontFamily uiFontFamily = UiFontFamily.system,
  Locale locale = const Locale('en'),
}) => _buildTheme(Brightness.dark, uiFontFamily: uiFontFamily, locale: locale);

ThemeData _buildTheme(
  Brightness brightness, {
  required UiFontFamily uiFontFamily,
  required Locale locale,
}) {
  const teal = Color(0xFF0F766E);
  const gold = Color(0xFFB7791F);

  final ColorScheme scheme;
  if (brightness == Brightness.light) {
    const ink = Color(0xFF14181F);
    const surface = Color(0xFFF5F7FA);
    const steel = Color(0xFF334155);
    scheme =
        ColorScheme.fromSeed(
          seedColor: teal,
          brightness: Brightness.light,
          primary: teal,
          secondary: gold,
          tertiary: steel,
          surface: surface,
          onSurface: ink,
        ).copyWith(
          // Pin the slate palette the widgets rely on, so light mode keeps
          // its exact pre-theming colors.
          surfaceContainerLowest: const Color(0xFFFFFFFF),
          surfaceContainerLow: const Color(0xFFF8FAFC),
          surfaceContainerHighest: const Color(0xFFF1F5F9),
          outline: const Color(0xFFCBD5E1),
          outlineVariant: const Color(0xFFE2E8F0),
          onSurfaceVariant: const Color(0xFF64748B),
          primaryContainer: const Color(0xFFE0F2F1),
          onPrimaryContainer: teal,
        );
  } else {
    scheme =
        ColorScheme.fromSeed(
          seedColor: teal,
          brightness: Brightness.dark,
          secondary: gold,
        ).copyWith(
          primary: const Color(0xFF2FB8A6),
          surface: const Color(0xFF11161D),
          onSurface: const Color(0xFFE2E8F0),
          surfaceContainerLowest: const Color(0xFF1A212B),
          surfaceContainerLow: const Color(0xFF161D26),
          surfaceContainerHighest: const Color(0xFF243040),
          outline: const Color(0xFF3D4A5C),
          outlineVariant: const Color(0xFF2E3A49),
          onSurfaceVariant: const Color(0xFF94A3B8),
          primaryContainer: const Color(0xFF134E48),
          onPrimaryContainer: const Color(0xFFA7F3EB),
        );
  }

  return ThemeData(
    useMaterial3: true,
    colorScheme: scheme,
    scaffoldBackgroundColor: scheme.surface,
    fontFamily: uiFontFamilyName(uiFontFamily, locale),
    fontFamilyFallback: uiFontFamily == UiFontFamily.system
        ? locale.languageCode == 'ja'
              ? const ['Yu Gothic UI', 'Microsoft YaHei UI']
              : const ['Microsoft YaHei UI', 'Yu Gothic UI']
        : null,
    appBarTheme: AppBarTheme(
      backgroundColor: scheme.surfaceContainerLowest,
      foregroundColor: scheme.onSurface,
      elevation: 0,
      centerTitle: false,
    ),
    cardTheme: CardThemeData(
      color: scheme.surfaceContainerLowest,
      elevation: 0,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(8),
        side: BorderSide(color: scheme.outlineVariant),
      ),
    ),
    dividerTheme: DividerThemeData(color: scheme.outlineVariant),
    filledButtonTheme: FilledButtonThemeData(
      style: FilledButton.styleFrom(
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
      ),
    ),
    outlinedButtonTheme: OutlinedButtonThemeData(
      style: OutlinedButton.styleFrom(
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
      ),
    ),
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: scheme.surfaceContainerLowest,
      border: OutlineInputBorder(borderRadius: BorderRadius.circular(8)),
      enabledBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(8),
        borderSide: BorderSide(color: scheme.outline),
      ),
      focusedBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(8),
        borderSide: BorderSide(color: scheme.primary, width: 1.5),
      ),
    ),
  );
}
