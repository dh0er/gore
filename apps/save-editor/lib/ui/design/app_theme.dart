import 'package:flutter/material.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';

const podkovaFontFamily = 'Podkova';
const notoSerifFontFamily = 'NotoSerif';
const notoSerifJpFontFamily = 'NotoSerifJP';
const notoSerifScFontFamily = 'NotoSerifSC';

/// Fallback faces for glyphs the primary face does not have.
///
/// The system font links missing CJK through the Windows UI faces. Bundled
/// fonts add the other Noto face, and Noto Serif itself whenever the primary
/// face does not cover Latin and Cyrillic. Noto Serif JP and Noto Serif SC
/// both draw Han, and the Windows UI faces do too, so this list never selects
/// the other CJK face. Game-text widgets use [gameScriptTextStyle] for that.
List<String>? uiFontFamilyFallback(
  UiFontFamily font,
  Locale uiLocale, {
  Locale? gameTextLocale,
}) {
  if (font == UiFontFamily.system) {
    return _systemFontFallback(uiLocale, gameTextLocale);
  }
  final primary = uiFontFamilyName(font, uiLocale);
  final fallback = <String>[];
  void add(String name) {
    if (name == primary || fallback.contains(name)) return;
    fallback.add(name);
  }

  add(scriptCoverageFont(uiLocale));
  if (gameTextLocale != null) add(scriptCoverageFont(gameTextLocale));
  add(notoSerifFontFamily);
  return fallback.isEmpty ? null : fallback;
}

List<String> _systemFontFallback(Locale uiLocale, Locale? gameTextLocale) {
  const ja = 'Yu Gothic UI';
  const hans = 'Microsoft YaHei UI';
  const hant = 'Microsoft JhengHei UI';

  String? face(Locale locale) => switch (locale.languageCode) {
    'ja' => ja,
    'zh' when locale.scriptCode == 'Hant' => hant,
    'zh' => hans,
    _ => null,
  };

  final ordered = <String>[];
  void add(String? name) {
    if (name != null && !ordered.contains(name)) ordered.add(name);
  }

  add(face(uiLocale));
  if (gameTextLocale != null) add(face(gameTextLocale));
  add(hans);
  add(hant);
  add(ja);
  return ordered;
}

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

/// Bundled face that covers [locale]'s script. Latin, Cyrillic and Greek stay
/// on Noto Serif; Japanese and both Chinese scripts use the CJK faces.
String scriptCoverageFont(Locale locale) => switch (locale.languageCode) {
  'ja' => notoSerifJpFontFamily,
  'zh' => notoSerifScFontFamily,
  _ => notoSerifFontFamily,
};

/// Game-text language for widgets that do not receive it as a parameter.
///
/// Absent in widget tests that pump a panel on its own. Those keep the theme
/// face. The running app provides it from the selected game-text language.
class GameTextScript extends InheritedWidget {
  const GameTextScript({super.key, required this.locale, required super.child});

  final Locale locale;

  static Locale? maybeOf(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<GameTextScript>()?.locale;

  @override
  bool updateShouldNotify(GameTextScript oldWidget) =>
      oldWidget.locale != locale;
}

/// Style for a game-text run when its script face is not the interface face.
///
/// Merge the result onto the text's own style. A null result means the scripts
/// share a face, so Podkova, Segoe, and that shared CJK face stay as the theme
/// set them. When [style] is set and the faces differ, [style] is returned with
/// the game-text family as the primary face.
TextStyle? gameScriptTextStyle(
  BuildContext context,
  Locale gameLocale, {
  TextStyle? style,
}) {
  final gameFont = scriptCoverageFont(gameLocale);
  final uiFont = scriptCoverageFont(Localizations.localeOf(context));
  if (gameFont == uiFont) return style;
  final override = TextStyle(
    fontFamily: gameFont,
    fontFamilyFallback: [
      uiFont,
      notoSerifFontFamily,
    ].where((font) => font != gameFont).toList(),
  );
  return (style ?? const TextStyle()).merge(override);
}

ThemeData buildGoresaveTheme({
  UiFontFamily uiFontFamily = UiFontFamily.system,
  Locale locale = const Locale('en'),
  Locale? gameTextLocale,
}) => _buildTheme(
  Brightness.light,
  uiFontFamily: uiFontFamily,
  locale: locale,
  gameTextLocale: gameTextLocale,
);

ThemeData buildGoresaveDarkTheme({
  UiFontFamily uiFontFamily = UiFontFamily.system,
  Locale locale = const Locale('en'),
  Locale? gameTextLocale,
}) => _buildTheme(
  Brightness.dark,
  uiFontFamily: uiFontFamily,
  locale: locale,
  gameTextLocale: gameTextLocale,
);

ThemeData _buildTheme(
  Brightness brightness, {
  required UiFontFamily uiFontFamily,
  required Locale locale,
  Locale? gameTextLocale,
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
    fontFamilyFallback: uiFontFamilyFallback(
      uiFontFamily,
      locale,
      gameTextLocale: gameTextLocale,
    ),
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
    // Attribute rows carry the game's own descriptions, which are whole
    // sentences. An unconstrained tooltip lays those out on ONE line and
    // stretches to the window edge; bound the width so they wrap into a block.
    tooltipTheme: const TooltipThemeData(
      constraints: BoxConstraints(maxWidth: 420),
      padding: EdgeInsets.symmetric(horizontal: 10, vertical: 8),
    ),
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
