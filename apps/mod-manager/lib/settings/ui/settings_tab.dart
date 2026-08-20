import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../app/domain/ui_settings.dart';
import '../../l10n/app_localizations.dart';
import '../../loc/game_lang.dart';
import 'update_settings.dart';

/// The Settings tab: appearance (theme mode + UI scale), game executable path,
/// app/game language, and the advanced-details switch. Holds no local state —
/// everything is wired to Riverpod providers so the panel can be rebuilt freely
/// on tab switches.
class SettingsTab extends ConsumerWidget {
  const SettingsTab({required this.gamePathFocusNode, super.key});

  final FocusNode gamePathFocusNode;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final gameExePath = ref.watch(gameExePathProvider);
    final currentLang = gameLangByCode(ref.watch(localeProvider));
    final themeMode = ref.watch(themeModeProvider);
    final uiScale = ref.watch(uiScaleProvider);
    final advancedDetails = ref.watch(advancedDetailsProvider);
    final textTheme = Theme.of(context).textTheme;

    return Align(
      alignment: Alignment.topCenter,
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 720),
        child: ListView(
          key: const ValueKey('settings-scroll-view'),
          padding: const EdgeInsets.all(24),
          children: [
            // --- Appearance (theme mode + UI scale) ---------------------
            Card(
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        const Icon(Icons.palette_outlined),
                        const SizedBox(width: 8),
                        Text(
                          l10n.appearanceTitle,
                          style: textTheme.titleMedium,
                        ),
                      ],
                    ),
                    const SizedBox(height: 16),
                    LayoutBuilder(
                      builder: (context, constraints) {
                        final selector = SegmentedButton<ThemeMode>(
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
                        );
                        final compact =
                            constraints.maxWidth < 680 ||
                            MediaQuery.textScalerOf(context).scale(1) > 1.35;
                        if (compact) {
                          return Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Text(l10n.theme, style: textTheme.labelLarge),
                              const SizedBox(height: 8),
                              SingleChildScrollView(
                                scrollDirection: Axis.horizontal,
                                child: selector,
                              ),
                            ],
                          );
                        }
                        return Row(
                          children: [
                            SizedBox(
                              width: 90,
                              child: Text(
                                l10n.theme,
                                style: textTheme.labelLarge,
                              ),
                            ),
                            selector,
                          ],
                        );
                      },
                    ),
                    const SizedBox(height: 16),
                    Row(
                      children: [
                        SizedBox(
                          width: 90,
                          child: Text(
                            l10n.uiScale,
                            style: textTheme.labelLarge,
                          ),
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
                          onPressed: () =>
                              ref.read(uiScaleProvider.notifier).reset(),
                        ),
                      ],
                    ),
                    const SizedBox(height: 4),
                    Text(l10n.zoomTip, style: textTheme.bodySmall),
                  ],
                ),
              ),
            ),

            // --- Game executable ----------------------------------------
            Card(
              child: ListTile(
                leading: const Icon(Icons.videogame_asset_outlined),
                title: Text(l10n.settingsGameExe),
                subtitle: Text(
                  gameExePath ?? '—',
                  maxLines: 3,
                  overflow: TextOverflow.ellipsis,
                ),
                trailing: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    if (gameExePath != null)
                      IconButton(
                        icon: const Icon(Icons.clear),
                        onPressed: () =>
                            ref.read(gameExePathProvider.notifier).clear(),
                      ),
                    Builder(
                      builder: (pickerContext) => OutlinedButton(
                        key: const ValueKey('settings-game-exe-pick'),
                        focusNode: gamePathFocusNode,
                        onFocusChange: (hasFocus) {
                          if (hasFocus) {
                            Scrollable.ensureVisible(
                              pickerContext,
                              alignment: 0.5,
                            );
                          }
                        },
                        onPressed: () async {
                          final group = XTypeGroup(
                            label: l10n.settingsGameExe,
                            extensions: const ['exe'],
                          );
                          final file = await openFile(
                            acceptedTypeGroups: [group],
                          );
                          if (file != null) {
                            ref
                                .read(gameExePathProvider.notifier)
                                .set(file.path);
                          }
                        },
                        child: Text(l10n.settingsGameExePick),
                      ),
                    ),
                  ],
                ),
              ),
            ),

            // --- Language -----------------------------------------------
            Card(
              child: ListTile(
                leading: const Icon(Icons.language_outlined),
                title: Text(l10n.settingsLanguage),
                trailing: DropdownButton<String>(
                  value: currentLang.code,
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

            // --- Updates -------------------------------------------------
            const UpdateSettingsCard(),

            // --- Advanced details ---------------------------------------
            Card(
              child: SwitchListTile(
                key: const ValueKey('settings-advanced-details'),
                secondary: const Icon(Icons.tune_outlined),
                title: Text(l10n.settingsAdvanced),
                subtitle: Text(l10n.settingsAdvancedHint),
                isThreeLine: true,
                value: advancedDetails,
                onChanged: (value) =>
                    ref.read(advancedDetailsProvider.notifier).set(value),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
