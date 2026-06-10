import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';

/// Appearance settings (theme mode, UI scale) shown in the Settings tab.
class AppearanceSettingsCard extends ConsumerWidget {
  const AppearanceSettingsCard({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final themeMode = ref.watch(themeModeProvider);
    final uiScale = ref.watch(uiScaleProvider);
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
                Text('Appearance', style: textTheme.titleMedium),
              ],
            ),
            const SizedBox(height: 16),
            Row(
              children: [
                SizedBox(
                  width: 90,
                  child: Text('Theme', style: textTheme.labelLarge),
                ),
                SegmentedButton<ThemeMode>(
                  segments: const [
                    ButtonSegment(
                      value: ThemeMode.light,
                      icon: Icon(Icons.light_mode_outlined),
                      label: Text('Light'),
                    ),
                    ButtonSegment(
                      value: ThemeMode.dark,
                      icon: Icon(Icons.dark_mode_outlined),
                      label: Text('Dark'),
                    ),
                    ButtonSegment(
                      value: ThemeMode.system,
                      icon: Icon(Icons.brightness_auto_outlined),
                      label: Text('System'),
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
                  child: Text('UI scale', style: textTheme.labelLarge),
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
                  tooltip: 'Reset zoom (Ctrl+0)',
                  icon: const Icon(Icons.restart_alt),
                  onPressed: () => ref.read(uiScaleProvider.notifier).reset(),
                ),
              ],
            ),
            const SizedBox(height: 4),
            Text(
              'Tip: Ctrl + / Ctrl - changes the zoom anywhere in the app.',
              style: textTheme.bodySmall,
            ),
          ],
        ),
      ),
    );
  }
}
