import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/loc/game_lang.dart';
import 'package:goresave/ui/design/app_theme.dart';

/// Appearance settings (theme mode, UI scale, languages) shown in the Settings
/// tab.
class AppearanceSettingsCard extends ConsumerWidget {
  const AppearanceSettingsCard({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final themeMode = ref.watch(themeModeProvider);
    final selectedUiFont = ref.watch(uiFontFamilyProvider);
    final uiScale = ref.watch(uiScaleProvider);
    // Normalize through the catalogs so an unknown persisted code maps to a
    // real entry; otherwise the dropdown asserts on a value with no item.
    final uiLang = uiLangByCode(ref.watch(localeProvider));
    final gameLang = gameLangByCode(ref.watch(gameTextLocaleProvider));
    final uiFont = effectiveUiFontFamily(selectedUiFont, uiLang.locale);
    final l10n = AppLocalizations.of(context);
    final textTheme = Theme.of(context).textTheme;
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                const Icon(Icons.palette_outlined),
                const SizedBox(width: 8),
                Text(l10n.appearanceTitle, style: textTheme.titleMedium),
              ],
            ),
            const SizedBox(height: 16),
            Row(
              children: [
                SizedBox(
                  width: 90,
                  child: Text(l10n.theme, style: textTheme.labelLarge),
                ),
                SegmentedButton<ThemeMode>(
                  segments: [
                    ButtonSegment(
                      value: ThemeMode.light,
                      icon: const Icon(Icons.light_mode_outlined),
                      label: Text(l10n.themeLight),
                    ),
                    ButtonSegment(
                      value: ThemeMode.dark,
                      icon: const Icon(Icons.dark_mode_outlined),
                      label: Text(l10n.themeDark),
                    ),
                    ButtonSegment(
                      value: ThemeMode.system,
                      icon: const Icon(Icons.brightness_auto_outlined),
                      label: Text(l10n.themeSystem),
                    ),
                  ],
                  selected: {themeMode},
                  onSelectionChanged: (selection) => ref
                      .read(themeModeProvider.notifier)
                      .setThemeMode(selection.first),
                ),
              ],
            ),
            const SizedBox(height: 16),
            _LanguageRow(
              label: l10n.language,
              value: uiLang.code,
              dropdownKey: const ValueKey('ui-language-dropdown'),
              items: [
                for (final lang in kUiLangs)
                  DropdownMenuItem(value: lang.code, child: Text(lang.endonym)),
              ],
              onChanged: (code) => selectUiLanguage(ref, code),
            ),
            const SizedBox(height: 16),
            _LanguageRow(
              label: l10n.gameTextLanguage,
              value: gameLang.code,
              dropdownKey: const ValueKey('game-text-language-dropdown'),
              items: [
                for (final lang in kGameLangs)
                  DropdownMenuItem(value: lang.code, child: Text(lang.endonym)),
              ],
              onChanged: (code) => ref
                  .read(gameTextLocaleProvider.notifier)
                  .setGameTextLocale(code),
            ),
            const SizedBox(height: 4),
            Text(l10n.gameTextLanguageHint, style: textTheme.bodySmall),
            const SizedBox(height: 16),
            Row(
              children: [
                SizedBox(
                  width: 90,
                  child: Text(l10n.uiFont, style: textTheme.labelLarge),
                ),
                Expanded(
                  child: Align(
                    alignment: Alignment.centerLeft,
                    child: DropdownButton<UiFontFamily>(
                      key: const ValueKey('ui-font-family-dropdown'),
                      value: uiFont,
                      onChanged: (font) {
                        if (font != null) {
                          ref.read(uiFontFamilyProvider.notifier).set(font);
                        }
                      },
                      items: [
                        for (final font in UiFontFamily.values)
                          if (uiFontFamilySupportedFor(font, uiLang.locale))
                            DropdownMenuItem(
                              value: font,
                              child: Text(
                                switch (font) {
                                  UiFontFamily.system => 'Segoe UI',
                                  UiFontFamily.podkova => 'Podkova',
                                  UiFontFamily.notoSerif => 'Noto Serif',
                                },
                                style: TextStyle(
                                  fontFamily: uiFontFamilyName(
                                    font,
                                    uiLang.locale,
                                  ),
                                ),
                              ),
                            ),
                      ],
                    ),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 16),
            Row(
              children: [
                SizedBox(
                  width: 90,
                  child: Text(l10n.uiScale, style: textTheme.labelLarge),
                ),
                Expanded(
                  child: Slider(
                    value: uiScale,
                    min: 0.5,
                    max: 2.0,
                    divisions: 30,
                    label: '${(uiScale * 100).round()}%',
                    onChanged: (value) =>
                        ref.read(uiScaleProvider.notifier).set(value),
                  ),
                ),
                SizedBox(
                  width: 48,
                  child: Text(
                    '${(uiScale * 100).round()}%',
                    textAlign: TextAlign.end,
                    style: textTheme.bodyMedium,
                  ),
                ),
                IconButton(
                  tooltip: l10n.resetZoomTooltip,
                  icon: const Icon(Icons.restart_alt),
                  onPressed: () => ref.read(uiScaleProvider.notifier).reset(),
                ),
              ],
            ),
            const SizedBox(height: 4),
            Text(l10n.zoomTip, style: textTheme.bodySmall),
          ],
        ),
      ),
    );
  }
}

class _LanguageRow extends StatelessWidget {
  const _LanguageRow({
    required this.label,
    required this.value,
    required this.items,
    required this.onChanged,
    required this.dropdownKey,
  });

  final String label;
  final String value;
  final List<DropdownMenuItem<String>> items;
  final ValueChanged<String> onChanged;
  final Key dropdownKey;

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        SizedBox(
          width: 132,
          child: Text(label, style: Theme.of(context).textTheme.labelLarge),
        ),
        Expanded(
          child: Align(
            alignment: Alignment.centerLeft,
            child: DropdownButton<String>(
              key: dropdownKey,
              value: value,
              onChanged: (code) {
                if (code != null) onChanged(code);
              },
              items: items,
            ),
          ),
        ),
      ],
    );
  }
}
