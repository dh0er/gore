import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../l10n/app_localizations.dart';
import '../../library/domain/conflicts_provider.dart';
import '../../library/domain/library_notifier.dart';
import '../../library/domain/models.dart';
import '../../library/ui/detail_panel.dart';

/// All conflicts across the loadout, grouped by their footprint target. Each
/// row shows the severity icon, the ordered mod chain, and the winner marked
/// (via [ConflictRow]).
class ConflictPanel extends ConsumerWidget {
  const ConflictPanel({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final library = ref.watch(libraryProvider);
    final conflictsAsync = ref.watch(conflictsProvider);

    return conflictsAsync.when(
      loading: () => const Padding(
        padding: EdgeInsets.all(16),
        child: Center(child: CircularProgressIndicator()),
      ),
      error: (err, _) => Padding(
        padding: const EdgeInsets.all(16),
        child: Text(
          '$err',
          style: theme.textTheme.bodySmall?.copyWith(
            color: theme.colorScheme.error,
          ),
        ),
      ),
      data: (conflicts) {
        if (conflicts.isEmpty) {
          return Padding(
            padding: const EdgeInsets.all(16),
            child: Text(
              l10n.noConflicts,
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          );
        }
        final order = [for (final e in library.loadout.entries) e.id];
        // Group by target, preserving first-seen order.
        final groups = <String, List<ConflictView>>{};
        for (final c in conflicts) {
          groups.putIfAbsent(c.target, () => []).add(c);
        }
        return ListView(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
          children: [
            for (final entry in groups.entries) ...[
              Padding(
                padding: const EdgeInsets.only(top: 8, bottom: 2),
                child: Text(
                  entry.key,
                  style: theme.textTheme.labelLarge?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
              ),
              for (final c in entry.value)
                ConflictRow(
                  conflict: c,
                  chain: orderConflictChain(c, order),
                  nameFor: (id) => library.modById(id)?.name ?? id,
                ),
            ],
          ],
        );
      },
    );
  }
}
