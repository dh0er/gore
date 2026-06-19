import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/app/domain/desktop_updater.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/l10n/app_localizations.dart';

/// Update settings (auto-check toggle, manual check) shown in the Settings
/// tab. Controls are disabled for builds that cannot update themselves
/// (dev runs and the portable zip).
class UpdateSettingsCard extends ConsumerWidget {
  const UpdateSettingsCard({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final autoCheck = ref.watch(autoUpdateCheckProvider);
    final available = isDesktopUpdaterAvailable;
    final textTheme = Theme.of(context).textTheme;
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                const Icon(Icons.system_update_alt_outlined),
                const SizedBox(width: 8),
                Text(l10n.updatesTitle, style: textTheme.titleMedium),
              ],
            ),
            const SizedBox(height: 8),
            SwitchListTile(
              contentPadding: EdgeInsets.zero,
              title: Text(l10n.checkForUpdatesAutomatically),
              value: autoCheck,
              onChanged: available
                  ? (enabled) {
                      ref.read(autoUpdateCheckProvider.notifier).set(enabled);
                      setAutoUpdateCheckEnabled(enabled);
                    }
                  : null,
            ),
            const SizedBox(height: 8),
            Row(
              children: [
                FilledButton.tonalIcon(
                  onPressed: available ? checkForUpdatesManually : null,
                  icon: const Icon(Icons.refresh),
                  label: Text(l10n.checkForUpdatesNow),
                ),
              ],
            ),
            if (!available) ...[
              const SizedBox(height: 8),
              Text(
                l10n.updatesPortableNotice,
                style: textTheme.bodySmall,
              ),
            ],
          ],
        ),
      ),
    );
  }
}
