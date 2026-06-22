import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/localization/domain/localization_controller.dart';
import 'package:goresave/features/localization/ui/localization_flow.dart';
import 'package:goresave/l10n/app_localizations.dart';

/// Settings-tab card that lets the user extract / refresh the localized game
/// text on demand. Mirrors [UpdateSettingsCard]'s layout. The extraction itself
/// is handled by [runLocalizationExtractFlow] (auto-detect + .lcache picker
/// fallback); this card just renders status and triggers it.
class LocalizationSettingsCard extends ConsumerWidget {
  const LocalizationSettingsCard({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final loc = ref.watch(localizationControllerProvider);
    final textTheme = Theme.of(context).textTheme;
    final scheme = Theme.of(context).colorScheme;

    final String statusLine;
    if (loc.present) {
      final ids = loc.idCount;
      final langs = loc.languageCount;
      statusLine = (ids != null && langs != null)
          ? l10n.gameTextExtractedWithCounts(ids, langs)
          : l10n.gameTextExtracted;
    } else {
      statusLine = l10n.gameTextNotExtracted;
    }

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                const Icon(Icons.translate_outlined),
                const SizedBox(width: 8),
                Text(l10n.gameTextTitle, style: textTheme.titleMedium),
              ],
            ),
            const SizedBox(height: 8),
            Text(statusLine, style: textTheme.bodySmall),
            const SizedBox(height: 12),
            Row(
              children: [
                FilledButton.tonalIcon(
                  onPressed: loc.isRunning
                      ? null
                      : () => runLocalizationExtractFlow(context, ref),
                  icon: loc.isRunning
                      ? const SizedBox(
                          width: 16,
                          height: 16,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : const Icon(Icons.refresh),
                  label: Text(
                    loc.isRunning
                        ? l10n.extracting
                        : l10n.extractRefreshLocalizedText,
                  ),
                ),
              ],
            ),
            if (loc.message != null) ...[
              const SizedBox(height: 8),
              Text(
                loc.message!,
                style: textTheme.bodySmall?.copyWith(
                  color: loc.phase == LocalizationPhase.error
                      ? scheme.error
                      : null,
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }
}
