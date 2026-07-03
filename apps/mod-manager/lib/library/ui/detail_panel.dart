import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../l10n/app_localizations.dart';
import '../domain/conflicts_provider.dart';
import '../domain/library_notifier.dart';
import '../domain/models.dart';
import 'mod_labels.dart';
import 'mod_list.dart';

/// How many footprint targets to list before collapsing into a "+N more" line.
const _kTargetCap = 50;

/// The right-hand detail pane for the currently selected mod: metadata rows,
/// its components (each with the footprint targets it claims, capped), and the
/// conflicts it participates in with the winning mod highlighted.
class DetailPanel extends ConsumerWidget {
  const DetailPanel({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final selectedId = ref.watch(selectedModProvider);
    final library = ref.watch(libraryProvider);
    final mod = selectedId == null ? null : library.modById(selectedId);

    if (mod == null) {
      return Center(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Text(
            l10n.tabMods,
            style: theme.textTheme.bodyMedium?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
        ),
      );
    }

    final conflicts = ref.watch(conflictsProvider);
    final order = [for (final e in library.loadout.entries) e.id];
    final myConflicts = conflictsForMod(
      conflicts.value ?? const [],
      mod.id,
    );

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Text(
          mod.name.isEmpty ? mod.id : mod.name,
          style: theme.textTheme.titleMedium,
        ),
        const SizedBox(height: 8),
        _MetaRow(label: 'kind', value: kindLabel(l10n, mod.kind)),
        if (mod.version != null)
          _MetaRow(label: 'version', value: mod.version!),
        if (mod.author != null) _MetaRow(label: 'author', value: mod.author!),
        if (mod.source != null) _MetaRow(label: 'source', value: mod.source!),
        if (mod.importedAt != null)
          _MetaRow(label: 'imported', value: mod.importedAt!),
        const Divider(height: 24),

        // --- Components -------------------------------------------------
        Text(l10n.componentsTitle, style: theme.textTheme.titleSmall),
        const SizedBox(height: 4),
        for (final c in mod.components) _ComponentRow(component: c, l10n: l10n),

        // --- Conflicts for this mod -------------------------------------
        if (myConflicts.isNotEmpty) ...[
          const Divider(height: 24),
          Text(
            l10n.conflictsTitle(myConflicts.length),
            style: theme.textTheme.titleSmall,
          ),
          const SizedBox(height: 4),
          for (final c in myConflicts)
            ConflictRow(
              conflict: c,
              chain: orderConflictChain(c, order),
              nameFor: (id) => library.modById(id)?.name ?? id,
            ),
        ],
      ],
    );
  }
}

class _MetaRow extends StatelessWidget {
  const _MetaRow({required this.label, required this.value});
  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 2),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 72,
            child: Text(
              label,
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ),
          Expanded(
            child: Text(value, style: theme.textTheme.bodySmall),
          ),
        ],
      ),
    );
  }
}

class _ComponentRow extends StatelessWidget {
  const _ComponentRow({required this.component, required this.l10n});
  final ComponentView component;
  final AppLocalizations l10n;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final targets = component.targets;
    final shown = targets.take(_kTargetCap).toList();
    final extra = targets.length - shown.length;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Icon(
                Icons.widgets_outlined,
                size: 14,
                color: theme.colorScheme.onSurfaceVariant,
              ),
              const SizedBox(width: 6),
              Expanded(
                child: Text(
                  '${component.kind} · ${component.displayLabel}',
                  style: theme.textTheme.bodySmall,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
            ],
          ),
          if (shown.isNotEmpty)
            Padding(
              padding: const EdgeInsets.only(left: 20, top: 2),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  for (final t in shown)
                    Text(
                      t,
                      style: theme.textTheme.labelSmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
                  if (extra > 0)
                    Text(
                      l10n.targetsMore(extra),
                      style: theme.textTheme.labelSmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                        fontStyle: FontStyle.italic,
                      ),
                    ),
                ],
              ),
            ),
        ],
      ),
    );
  }
}

/// One conflict: its kind + target and the ordered mod chain, with the winner
/// rendered bold and tagged. Reused by the detail panel and conflict panel.
class ConflictRow extends StatelessWidget {
  const ConflictRow({
    super.key,
    required this.conflict,
    required this.chain,
    required this.nameFor,
  });

  final ConflictView conflict;
  final OrderedConflictChain chain;
  final String Function(String id) nameFor;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              SeverityIcon(severity: conflict.severity),
              const SizedBox(width: 6),
              Expanded(
                child: Text(
                  '${conflict.kind} · ${conflict.target}',
                  style: theme.textTheme.bodySmall,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
            ],
          ),
          Padding(
            padding: const EdgeInsets.only(left: 20, top: 2),
            child: Wrap(
              spacing: 4,
              runSpacing: 2,
              crossAxisAlignment: WrapCrossAlignment.center,
              children: [
                for (final id in chain.modIds)
                  _ModChainChip(
                    name: nameFor(id),
                    winner: chain.isWinner(id),
                    winnerTag: l10n.conflictWinner,
                  ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _ModChainChip extends StatelessWidget {
  const _ModChainChip({
    required this.name,
    required this.winner,
    required this.winnerTag,
  });
  final String name;
  final bool winner;
  final String winnerTag;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 1),
      decoration: BoxDecoration(
        color: winner ? scheme.tertiaryContainer : scheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(4),
      ),
      child: Text.rich(
        TextSpan(
          children: [
            TextSpan(
              text: name,
              style: TextStyle(
                fontSize: 11,
                fontWeight: winner ? FontWeight.bold : FontWeight.normal,
                color: winner
                    ? scheme.onTertiaryContainer
                    : scheme.onSurfaceVariant,
              ),
            ),
            if (winner)
              TextSpan(
                text: '  $winnerTag',
                style: TextStyle(
                  fontSize: 10,
                  fontWeight: FontWeight.bold,
                  color: scheme.onTertiaryContainer,
                ),
              ),
          ],
        ),
      ),
    );
  }
}

/// Severity glyph for a conflict, colored by severity (hard=error,
/// soft=amber, info=grey, unknown=grey outline).
class SeverityIcon extends StatelessWidget {
  const SeverityIcon({super.key, required this.severity});
  final String severity;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final (IconData icon, Color color) = switch (severity) {
      'hard' => (Icons.error_outline, scheme.error),
      'soft' => (Icons.warning_amber_rounded, Colors.amber.shade700),
      'info' => (Icons.info_outline, scheme.onSurfaceVariant),
      _ => (Icons.help_outline, scheme.onSurfaceVariant),
    };
    return Icon(icon, size: 14, color: color);
  }
}
