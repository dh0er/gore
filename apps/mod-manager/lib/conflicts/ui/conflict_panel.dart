import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../app/domain/ui_settings.dart';
import '../../core/technical_details.dart';
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
    final incompleteCoverage = ref.watch(
      enabledFootprintKnowledgeIncompleteProvider,
    );
    final advanced = ref.watch(advancedDetailsProvider);

    if (!library.authoritative) {
      return Padding(
        padding: const EdgeInsets.all(16),
        child: Text(
          l10n.conflictsUnverified,
          style: theme.textTheme.bodySmall?.copyWith(
            color: theme.colorScheme.error,
          ),
        ),
      );
    }

    return conflictsAsync.when(
      loading: () => const Padding(
        padding: EdgeInsets.all(16),
        child: Center(child: CircularProgressIndicator()),
      ),
      error: (error, _) {
        return Padding(
          padding: const EdgeInsetsDirectional.fromSTEB(16, 4, 4, 4),
          child: Row(
            children: [
              Expanded(
                child: Text(
                  l10n.conflictsUnavailable,
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.error,
                  ),
                ),
              ),
              TechnicalDetailsIconButton(
                key: const ValueKey('conflict-technical-details-action'),
                detail: '$error',
              ),
            ],
          ),
        );
      },
      data: (conflicts) {
        if (conflicts.isEmpty) {
          return ListView(
            padding: const EdgeInsets.all(16),
            children: [
              Text(
                l10n.noConflicts,
                style: theme.textTheme.bodySmall?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
              const SizedBox(height: 12),
              _ConflictKnowledgeNote(
                incompleteCoverage: incompleteCoverage,
                advanced: advanced,
                l10n: l10n,
              ),
            ],
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
            const SizedBox(height: 8),
            _ConflictKnowledgeNote(
              incompleteCoverage: incompleteCoverage,
              advanced: advanced,
              l10n: l10n,
            ),
          ],
        );
      },
    );
  }
}

/// Footnote under the conflict list.
///
/// The incomplete-knowledge warning is actionable, so it always shows when it
/// applies. The two standing caveats (which direction load order runs, what the
/// listed targets do and don't prove) are background reading — they only show
/// while [advancedDetailsProvider] is on, so the plain view stays quiet when
/// there is nothing to warn about.
class _ConflictKnowledgeNote extends StatelessWidget {
  const _ConflictKnowledgeNote({
    required this.incompleteCoverage,
    required this.advanced,
    required this.l10n,
  });

  final bool incompleteCoverage;
  final bool advanced;
  final AppLocalizations l10n;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final lines = [
      if (incompleteCoverage) l10n.conflictCoverageIncomplete,
      if (advanced) l10n.loadOrderDirection,
      if (advanced) l10n.footprintCoverageScope,
    ];
    if (lines.isEmpty) {
      return const SizedBox.shrink(key: ValueKey('conflict-knowledge-note'));
    }
    final foreground = incompleteCoverage
        ? scheme.onTertiaryContainer
        : scheme.onSurfaceVariant;
    return Semantics(
      key: const ValueKey('conflict-knowledge-note'),
      container: true,
      label: lines.join(' '),
      child: ExcludeSemantics(
        child: Container(
          padding: const EdgeInsets.all(8),
          decoration: BoxDecoration(
            color: incompleteCoverage
                ? scheme.tertiaryContainer
                : scheme.surfaceContainerHighest,
            borderRadius: BorderRadius.circular(6),
          ),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Icon(
                incompleteCoverage
                    ? Icons.warning_amber_rounded
                    : Icons.info_outline,
                size: 16,
                color: foreground,
              ),
              const SizedBox(width: 8),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    for (var i = 0; i < lines.length; i++) ...[
                      if (i > 0) const SizedBox(height: 2),
                      Text(
                        lines[i],
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: foreground,
                          fontWeight: incompleteCoverage && i == 0
                              ? FontWeight.w600
                              : null,
                        ),
                      ),
                    ],
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
