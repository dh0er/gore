import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/loc/game_lang.dart';

/// Appearance settings (theme mode, UI scale, language) shown in the Settings
/// tab.
class AppearanceSettingsCard extends ConsumerWidget {
  const AppearanceSettingsCard({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final themeMode = ref.watch(themeModeProvider);
    final uiScale = ref.watch(uiScaleProvider);
    final localeCode = ref.watch(localeProvider);
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
            Row(
              children: [
                SizedBox(
                  width: 90,
                  child: Text(l10n.language, style: textTheme.labelLarge),
                ),
                Expanded(
                  child: Align(
                    alignment: Alignment.centerLeft,
                    child: DropdownButton<String>(
                      value: localeCode,
                      onChanged: (code) {
                        if (code != null) {
                          ref.read(localeProvider.notifier).setLocale(code);
                        }
                      },
                      items: [
                        for (final lang in kGameLangs)
                          DropdownMenuItem(
                            value: lang.code,
                            child: Text(lang.endonym),
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
            Text(
              l10n.zoomTip,
              style: textTheme.bodySmall,
            ),
          ],
        ),
      ),
    );
  }
}
