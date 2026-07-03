import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../app/domain/ui_settings.dart';
import '../../l10n/app_localizations.dart';
import '../../loc/game_lang.dart';

/// The Settings tab: game executable path + app/game language. Holds no local
/// state — everything is wired to Riverpod providers so the panel can be
/// rebuilt freely on tab switches.
class SettingsTab extends ConsumerWidget {
  const SettingsTab({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final gameExePath = ref.watch(gameExePathProvider);
    final currentLang = gameLangByCode(ref.watch(localeProvider));

    return Align(
      alignment: Alignment.topCenter,
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 720),
        child: ListView(
          padding: const EdgeInsets.all(24),
          children: [
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
                    OutlinedButton(
                      onPressed: () async {
                        final group = XTypeGroup(
                          label: l10n.settingsGameExe,
                          extensions: const ['exe'],
                        );
                        final file =
                            await openFile(acceptedTypeGroups: [group]);
                        if (file != null) {
                          ref
                              .read(gameExePathProvider.notifier)
                              .set(file.path);
                        }
                      },
                      child: Text(l10n.settingsGameExePick),
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
          ],
        ),
      ),
    );
  }
}
