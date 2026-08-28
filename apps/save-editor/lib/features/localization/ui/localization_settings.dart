import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/editor/domain/item_icon_catalog.dart';
import 'package:goresave/features/localization/domain/localization_controller.dart';
import 'package:goresave/features/localization/ui/localization_flow.dart';
import 'package:goresave/l10n/app_localizations.dart';

/// Settings surface for the game-owned assets the editor prepares locally.
///
/// Text extraction can fall back to an explicit `.lcache` picker. Item images
/// stay enhancement-only and prepare automatically; their action is a manual
/// cache verification/retry rather than a prerequisite for using the editor.
class GameDataSettingsCard extends ConsumerWidget {
  const GameDataSettingsCard({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final localization = ref.watch(localizationControllerProvider);
    final itemImages = ref.watch(itemIconCatalogProvider);
    final textTheme = Theme.of(context).textTheme;
    final scheme = Theme.of(context).colorScheme;

    final String textStatus;
    if (localization.present) {
      final ids = localization.idCount;
      final languages = localization.languageCount;
      textStatus = ids != null && languages != null
          ? l10n.gameTextExtractedWithCounts(ids, languages)
          : l10n.gameTextExtracted;
    } else {
      textStatus = l10n.gameTextNotExtracted;
    }

    final itemCount = itemImages.value?.pathByItemId.length ?? 0;
    final itemStatus = itemCount > 0
        ? l10n.itemImagesReady(itemCount)
        : itemImages.isLoading
        ? l10n.preparing
        : l10n.itemImagesUnavailable;

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                const Icon(Icons.storage_outlined),
                const SizedBox(width: 8),
                Text(l10n.gameDataTitle, style: textTheme.titleMedium),
              ],
            ),
            const SizedBox(height: 8),
            _GameDataRow(
              icon: Icons.translate_outlined,
              title: l10n.gameTextTitle,
              status: textStatus,
              message: localization.message,
              messageColor: localization.phase == LocalizationPhase.error
                  ? scheme.error
                  : null,
              action: FilledButton.tonalIcon(
                onPressed: localization.isRunning
                    ? null
                    : () => runLocalizationExtractFlow(context, ref),
                icon: localization.isRunning
                    ? const _ButtonProgress()
                    : const Icon(Icons.refresh),
                label: Text(
                  localization.isRunning
                      ? l10n.extracting
                      : l10n.extractRefreshLocalizedText,
                ),
              ),
            ),
            const Divider(height: 32),
            _GameDataRow(
              icon: Icons.image_outlined,
              title: l10n.itemImagesTitle,
              status: itemStatus,
              action: FilledButton.tonalIcon(
                onPressed: itemImages.isLoading
                    ? null
                    : () {
                        ref
                            .read(itemIconCatalogReloadProvider.notifier)
                            .state++;
                      },
                icon: itemImages.isLoading
                    ? const _ButtonProgress()
                    : const Icon(Icons.refresh),
                label: Text(
                  itemImages.isLoading
                      ? l10n.preparing
                      : l10n.checkRefreshItemImages,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _GameDataRow extends StatelessWidget {
  const _GameDataRow({
    required this.icon,
    required this.title,
    required this.status,
    required this.action,
    this.message,
    this.messageColor,
  });

  final IconData icon;
  final String title;
  final String status;
  final Widget action;
  final String? message;
  final Color? messageColor;

  @override
  Widget build(BuildContext context) {
    final details = Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Icon(icon, size: 20),
        const SizedBox(width: 10),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(title, style: Theme.of(context).textTheme.titleSmall),
              const SizedBox(height: 4),
              Text(status, style: Theme.of(context).textTheme.bodySmall),
              if (message case final message?) ...[
                const SizedBox(height: 4),
                Text(
                  message,
                  style: Theme.of(
                    context,
                  ).textTheme.bodySmall?.copyWith(color: messageColor),
                ),
              ],
            ],
          ),
        ),
      ],
    );

    return LayoutBuilder(
      builder: (context, constraints) {
        if (constraints.maxWidth < 680) {
          return Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [details, const SizedBox(height: 12), action],
          );
        }
        return Row(
          crossAxisAlignment: CrossAxisAlignment.center,
          children: [
            Expanded(child: details),
            const SizedBox(width: 16),
            action,
          ],
        );
      },
    );
  }
}

class _ButtonProgress extends StatelessWidget {
  const _ButtonProgress();

  @override
  Widget build(BuildContext context) {
    return const SizedBox(
      width: 16,
      height: 16,
      child: CircularProgressIndicator(strokeWidth: 2),
    );
  }
}
