import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/localization/domain/localization_controller.dart';
import 'package:goresave/features/localization/ui/localization_flow.dart';

/// Settings-tab card that lets the user extract / refresh the localized game
/// text on demand. Mirrors [UpdateSettingsCard]'s layout. The extraction itself
/// is handled by [runLocalizationExtractFlow] (auto-detect + .lcache picker
/// fallback); this card just renders status and triggers it.
class LocalizationSettingsCard extends ConsumerWidget {
  const LocalizationSettingsCard({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final loc = ref.watch(localizationControllerProvider);
    final textTheme = Theme.of(context).textTheme;
    final scheme = Theme.of(context).colorScheme;

    final String statusLine;
    if (loc.present) {
      final ids = loc.idCount;
      final langs = loc.languageCount;
      statusLine = (ids != null && langs != null)
          ? 'Extracted: $ids ids across $langs languages.'
          : 'Localized game text is extracted.';
    } else {
      statusLine = 'Localized game text is not extracted yet.';
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
                Text('Game text', style: textTheme.titleMedium),
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
                        ? 'Extracting…'
                        : 'Extract / refresh localized text',
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
