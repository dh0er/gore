import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../l10n/app_localizations.dart';
import '../../preflight/domain/preflight_notifier.dart';
import '../../status/domain/status_notifier.dart';
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
  const DetailPanel({
    super.key,
    this.queueImportFocusAfterRemove,
    this.queueRefreshFocusAfterRemove,
  });

  final ValueChanged<String>? queueImportFocusAfterRemove;
  final ValueChanged<String>? queueRefreshFocusAfterRemove;

  void _announceRemove(BuildContext context, String message) {
    final messenger = ScaffoldMessenger.of(context);
    messenger.hideCurrentSnackBar();
    messenger.showSnackBar(
      SnackBar(
        content: Text(key: const ValueKey('remove-mod-feedback'), message),
      ),
    );
  }

  Future<void> _confirmRemove(
    BuildContext context,
    WidgetRef ref,
    ModEntryMetaView mod,
  ) async {
    final l10n = AppLocalizations.of(context);
    final displayName = mod.name.isEmpty ? mod.id : mod.name;
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        scrollable: true,
        title: Text(l10n.removeModAction),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(l10n.removeModConfirm(displayName)),
            const SizedBox(height: 12),
            Text(l10n.removeModDeploymentHint),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(dialogContext, false),
            child: Text(l10n.commonCancel),
          ),
          FilledButton(
            style: FilledButton.styleFrom(
              backgroundColor: Theme.of(context).colorScheme.error,
              foregroundColor: Theme.of(context).colorScheme.onError,
            ),
            onPressed: () => Navigator.pop(dialogContext, true),
            child: Text(l10n.removeModAction),
          ),
        ],
      ),
    );
    if (confirmed != true || !context.mounted) return;

    // The button is disabled while a library call is running. Check once more
    // after the dialog closes so a mutation that started while the dialog was
    // open cannot overlap this destructive call.
    final before = ref.read(libraryProvider);
    if (before.busy ||
        !before.authoritative ||
        ref.read(statusProvider).busy ||
        ref.read(preflightProvider).busy ||
        ref.read(conflictsProvider).isLoading ||
        before.modById(mod.id) == null) {
      return;
    }

    final wasSelectedAtStart = ref.read(selectedModProvider) == mod.id;
    await ref.read(libraryProvider.notifier).remove(mod.id);
    if (!context.mounted) return;

    // The authoritative reload decides whether the entry still exists. A
    // native error may coexist with a removed entry after partial success, so
    // clear stale selection but announce that outcome as a warning, not success.
    final after = ref.read(libraryProvider);
    bool focusStillBelongsToRemovedMod() {
      final selected = ref.read(selectedModProvider);
      return wasSelectedAtStart && (selected == null || selected == mod.id);
    }

    if (after.modById(mod.id) == null) {
      if (!after.authoritative) {
        if (focusStillBelongsToRemovedMod()) {
          queueRefreshFocusAfterRemove?.call(mod.id);
        }
        if (after.error != null) {
          _announceRemove(context, l10n.removeModOutcomeUnknown(displayName));
        }
        return;
      }
      if (ref.read(selectedModProvider) == mod.id) {
        ref.read(selectedModProvider.notifier).state = null;
      }
      if (after.error != null) {
        if (focusStillBelongsToRemovedMod()) {
          queueRefreshFocusAfterRemove?.call(mod.id);
        }
        _announceRemove(context, l10n.removeModPartialFailure(displayName));
      } else {
        if (focusStillBelongsToRemovedMod()) {
          queueImportFocusAfterRemove?.call(mod.id);
        }
        _announceRemove(context, l10n.removeModSuccess(displayName));
      }
    } else if (after.error != null) {
      _announceRemove(context, l10n.removeModFailed(displayName));
    }
  }

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
    final status = ref.watch(statusProvider);
    final preflight = ref.watch(preflightProvider);
    final removeBlocked =
        library.busy ||
        !library.authoritative ||
        status.busy ||
        preflight.busy ||
        conflicts.isLoading;
    final order = [for (final e in library.loadout.entries) e.id];
    final myConflicts = conflictsForMod(conflicts.value ?? const [], mod.id);

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Text(
          mod.name.isEmpty ? mod.id : mod.name,
          style: theme.textTheme.titleMedium,
        ),
        const SizedBox(height: 8),
        _MetaRow(label: l10n.modDetailKind, value: kindLabel(l10n, mod.kind)),
        if (mod.version != null)
          _MetaRow(label: l10n.modDetailVersion, value: mod.version!),
        if (mod.author != null)
          _MetaRow(label: l10n.modDetailAuthor, value: mod.author!),
        if (mod.source != null)
          _MetaRow(label: l10n.modDetailSource, value: mod.source!),
        if (mod.importedAt != null)
          _MetaRow(label: l10n.modDetailImported, value: mod.importedAt!),
        const Divider(height: 24),

        // --- Components -------------------------------------------------
        Text(l10n.componentsTitle, style: theme.textTheme.titleSmall),
        const SizedBox(height: 4),
        for (var i = 0; i < mod.components.length; i++)
          _ComponentRow(component: mod.components[i], index: i, l10n: l10n),

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
        const Divider(height: 24),
        Align(
          alignment: AlignmentDirectional.centerStart,
          child: OutlinedButton.icon(
            key: const ValueKey('remove-mod-action'),
            onPressed: removeBlocked
                ? null
                : () => _confirmRemove(context, ref, mod),
            icon: const Icon(Icons.delete_outline),
            label: Text(l10n.removeModAction),
            style: OutlinedButton.styleFrom(
              foregroundColor: theme.colorScheme.error,
              side: BorderSide(color: theme.colorScheme.error),
            ),
          ),
        ),
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
          Expanded(child: Text(value, style: theme.textTheme.bodySmall)),
        ],
      ),
    );
  }
}

class _ComponentRow extends StatelessWidget {
  const _ComponentRow({
    required this.component,
    required this.index,
    required this.l10n,
  });
  final ComponentView component;
  final int index;
  final AppLocalizations l10n;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final localizedKind = componentKindLabel(l10n, component.kind);
    final displayLabel = component.displayLabel;
    final heading = displayLabel == component.kind
        ? localizedKind
        : '$localizedKind · $displayLabel';
    final targets = <String>[
      ...component.targets,
      if (component.rawFileTarget case final target?) _rawTargetLabel(target),
    ];
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
                  heading,
                  style: theme.textTheme.bodySmall,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
              const SizedBox(width: 6),
              _FootprintCoverageBadge(
                coverage: component.coverage,
                index: index,
                l10n: l10n,
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

String _rawTargetLabel(RawFileTargetView target) {
  if (target.kind == 'bank') {
    final name = target.bankName;
    return name == null ? target.kind : 'bank:$name';
  }
  return target.kind;
}

class _FootprintCoverageBadge extends StatelessWidget {
  const _FootprintCoverageBadge({
    required this.coverage,
    required this.index,
    required this.l10n,
  });

  final FootprintCoverage coverage;
  final int index;
  final AppLocalizations l10n;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final description = footprintCoverageLabel(l10n, coverage);
    final label = footprintCoverageShortLabel(l10n, coverage);
    final (Color background, Color foreground) = switch (coverage) {
      FootprintCoverage.exact => (
        scheme.secondaryContainer,
        scheme.onSecondaryContainer,
      ),
      FootprintCoverage.partial => (
        scheme.tertiaryContainer,
        scheme.onTertiaryContainer,
      ),
      FootprintCoverage.advisory => (
        scheme.surfaceContainerHighest,
        scheme.onSurfaceVariant,
      ),
      FootprintCoverage.opaque => (
        scheme.errorContainer,
        scheme.onErrorContainer,
      ),
    };
    return Tooltip(
      message: description,
      excludeFromSemantics: true,
      child: Semantics(
        key: ValueKey('component-footprint-coverage-$index'),
        container: true,
        label: description,
        child: ExcludeSemantics(
          child: Container(
            padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 1),
            decoration: BoxDecoration(
              color: background,
              borderRadius: BorderRadius.circular(4),
            ),
            child: Text(
              label,
              style: Theme.of(context).textTheme.labelSmall?.copyWith(
                color: foreground,
                fontWeight: FontWeight.w600,
              ),
            ),
          ),
        ),
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
                  '${conflictKindLabel(l10n, conflict.kind)} · '
                  '${conflict.target}',
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
        color: winner
            ? scheme.tertiaryContainer
            : scheme.surfaceContainerHighest,
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
