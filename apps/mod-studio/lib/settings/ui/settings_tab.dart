import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path/path.dart' as p;

import '../../app/domain/ui_settings.dart';
import '../../l10n/app_localizations.dart';
import '../../loc/domain/loc_notifier.dart';
import '../../loc/game_lang.dart';
import '../../loc/ui/loc_extract_flow.dart';

/// The Settings tab: scrollable, centred column of Material sections for the
/// game-data source, localized-text extraction, the game executable path, and
/// the app/game language. Holds no local state — everything is wired to
/// Riverpod providers so the panel can be rebuilt freely on tab switches.
class SettingsTab extends ConsumerWidget {
  const SettingsTab({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;

    final dumpPath = ref.watch(dumpPathProvider);
    final gameExePath = ref.watch(gameExePathProvider);
    final locRunning = ref.watch(locProvider).isRunning;
    final currentLang = gameLangByCode(ref.watch(localeProvider)).code;

    Widget sectionTitle(String text) => Padding(
          padding: const EdgeInsets.fromLTRB(4, 8, 4, 4),
          child: Text(
            text,
            style: theme.textTheme.titleSmall?.copyWith(
              color: scheme.primary,
            ),
          ),
        );

    return Align(
      alignment: Alignment.topCenter,
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 720),
        child: ListView(
          padding: const EdgeInsets.all(24),
          children: [
            // --- Game data source ---------------------------------------
            sectionTitle(l10n.settingsDataSourceSection),
            Card(
              child: Column(
                children: [
                  ListTile(
                    leading: Icon(
                      dumpPath != null
                          ? Icons.dataset
                          : Icons.dataset_outlined,
                      color: dumpPath != null ? scheme.primary : null,
                    ),
                    title: Text(l10n.loadGameDataDump),
                    subtitle: Text(
                      dumpPath != null
                          ? p.basename(dumpPath)
                          : l10n.loadGameDataDumpSubtitle,
                    ),
                    trailing: const Icon(Icons.upload_file),
                    onTap: () async {
                      final group = XTypeGroup(
                        label: l10n.gameDataFileGroupLabel,
                        extensions: const ['json'],
                      );
                      final file =
                          await openFile(acceptedTypeGroups: [group]);
                      if (file != null) {
                        ref.read(dumpPathProvider.notifier).set(file.path);
                      }
                    },
                  ),
                  const Divider(height: 1),
                  ListTile(
                    leading: const Icon(Icons.restore),
                    title: Text(l10n.useBundledData),
                    subtitle: Text(
                      dumpPath != null
                          ? p.basename(dumpPath)
                          : l10n.alreadyBundled,
                    ),
                    enabled: dumpPath != null,
                    onTap: dumpPath != null
                        ? () => ref.read(dumpPathProvider.notifier).clear()
                        : null,
                  ),
                ],
              ),
            ),

            // --- Localized text -----------------------------------------
            sectionTitle(l10n.settingsLocalizationSection),
            Card(
              child: ListTile(
                leading: const Icon(Icons.translate),
                title: Text(l10n.extractLocalizedText),
                trailing: FilledButton.icon(
                  icon: const Icon(Icons.translate, size: 18),
                  label: Text(l10n.extractLocalizedText),
                  onPressed: locRunning
                      ? null
                      : () => runLocExtractFlow(context, ref),
                ),
              ),
            ),

            // --- Game executable ----------------------------------------
            Card(
              child: ListTile(
                leading: const Icon(Icons.videogame_asset_outlined),
                title: Text(l10n.gameExecutable),
                subtitle: Text(
                  gameExePath ?? l10n.gameExecutableSubtitle,
                  maxLines: 3,
                  overflow: TextOverflow.ellipsis,
                ),
                trailing: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    if (gameExePath != null)
                      IconButton(
                        icon: const Icon(Icons.clear),
                        tooltip: l10n.gameExecutableNotSet,
                        onPressed: () =>
                            ref.read(gameExePathProvider.notifier).clear(),
                      ),
                    OutlinedButton(
                      onPressed: () async {
                        final group = XTypeGroup(
                          label: l10n.gameExecutable,
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
                      child: Text(l10n.chooseGameExecutable),
                    ),
                  ],
                ),
              ),
            ),

            // --- Language -----------------------------------------------
            sectionTitle(l10n.language),
            Card(
              child: Column(
                children: [
                  for (final lang in kGameLangs)
                    ListTile(
                      title: Text(lang.endonym),
                      trailing: lang.code == currentLang
                          ? Icon(Icons.check, color: scheme.primary)
                          : null,
                      selected: lang.code == currentLang,
                      onTap: () => ref
                          .read(localeProvider.notifier)
                          .setLocale(lang.code),
                    ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}
