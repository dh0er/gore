import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../app/domain/ui_settings.dart';
import '../../core/diagnostic_text.dart';
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

/// Readable local date+time for an ISO-8601 import timestamp.
///
/// Native records full precision (`2026-08-13T13:53:55.883181Z`), which is a
/// machine format: a reader wants to know the day, not the microsecond. Falls
/// back to the raw value when it cannot be parsed, so a future format is shown
/// rather than dropped.
String formatImportedAt(
  MaterialLocalizations material,
  String raw, {
  required bool alwaysUse24HourFormat,
}) {
  final parsed = DateTime.tryParse(raw);
  if (parsed == null) return raw;
  final local = parsed.toLocal();
  final time = material.formatTimeOfDay(
    TimeOfDay.fromDateTime(local),
    alwaysUse24HourFormat: alwaysUse24HourFormat,
  );
  // Short, not medium: medium drops the year, and a mod imported
  // last year would then read as if it arrived last week.
  return '${material.formatShortDate(local)}, $time';
}

/// The right-hand detail pane for the currently selected mod: metadata rows,
/// what it changes, and the conflicts it participates in with the winning mod
/// highlighted. The technical layer of a component — the footprint targets it
/// claims (capped) and its coverage grade — only appears while
/// [advancedDetailsProvider] is on.
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
            l10n.detailEmptyHint,
            textAlign: TextAlign.center,
            style: theme.textTheme.bodyMedium?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
        ),
      );
    }

    final conflicts = ref.watch(conflictsProvider);
    final advanced = ref.watch(advancedDetailsProvider);
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
        // How the mod was packaged is a fact about the download, not about the
        // game; the component rows below already say what it changes.
        if (advanced)
          _MetaRow(label: l10n.modDetailKind, value: kindLabel(l10n, mod.kind)),
        if (mod.version != null)
          _MetaRow(label: l10n.modDetailVersion, value: mod.version!),
        if (mod.author != null)
          _MetaRow(label: l10n.modDetailAuthor, value: mod.author!),
        if (advanced && mod.source != null)
          _MetaRow(
            label: l10n.modDetailSource,
            value: displayPath(mod.source!),
          ),
        if (mod.importedAt != null)
          _MetaRow(
            label: l10n.modDetailImported,
            value: formatImportedAt(
              MaterialLocalizations.of(context),
              mod.importedAt!,
              alwaysUse24HourFormat: MediaQuery.of(
                context,
              ).alwaysUse24HourFormat,
            ),
          ),
        const Divider(height: 24),

        // --- Components -------------------------------------------------
        Text(l10n.componentsTitle, style: theme.textTheme.titleSmall),
        const SizedBox(height: 4),
        for (var i = 0; i < mod.components.length; i++)
          _ComponentRow(
            component: mod.components[i],
            index: i,
            l10n: l10n,
            advanced: advanced,
          ),

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
    required this.advanced,
  });
  final ComponentView component;
  final int index;
  final AppLocalizations l10n;

  /// Plain view shows only what a component is. The raw target list and the
  /// coverage grade are the technical layer and stay hidden until asked for.
  final bool advanced;

  /// One line naming what this component is, specific enough to tell two rows
  /// of the same kind apart.
  ///
  /// A `raw_file` replaces one game-wide file wholesale, so it is named by its
  /// destination ("all game text") rather than by the word "file". Everything
  /// else is named by the kind plus whatever identifies this instance: the
  /// script name, or the file name it ships (the full relative path only in the
  /// advanced view, where the path is the point).
  String _heading() {
    if (component.rawFileTarget case final target?) {
      if (rawFileTargetLabel(l10n, target) case final destination?) {
        return advanced && component.rel != null
            ? '$destination · ${component.rel}'
            : destination;
      }
    }
    final label = advanced
        ? componentKindLabel(l10n, component.kind)
        : componentPlainLabel(l10n, component);
    final detail = advanced
        ? component.displayLabel
        : component.name ?? _fileName(component.rel);
    return detail == null || detail == component.kind
        ? label
        : '$label · $detail';
  }

  /// Last path segment of [path]; null when there is nothing useful to show.
  static String? _fileName(String? path) {
    if (path == null || path.isEmpty) return null;
    final segments = path.split(RegExp(r'[/\\]'))
      ..removeWhere((segment) => segment.isEmpty);
    return segments.isEmpty ? null : segments.last;
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final heading = _heading();
    final targets = advanced ? component.targets : const <String>[];
    final shown = targets.take(_kTargetCap).toList();
    final extra = targets.length - shown.length;
    final muted = theme.textTheme.labelSmall?.copyWith(
      color: theme.colorScheme.onSurfaceVariant,
    );
    // Opaque has nothing to list, so its note stands alone; the other grades
    // only make sense as the heading of the list they describe.
    final showTargetsNote =
        advanced &&
        (shown.isNotEmpty || component.coverage == FootprintCoverage.opaque);
    // One SelectionArea over the whole row: headings and paths stay copyable
    // without SelectableText's private Scrollable, which would make "the
    // DetailPanel's scrollable" ambiguous for callers and tests alike.
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: SelectionArea(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Padding(
                  padding: const EdgeInsets.only(top: 2),
                  child: Icon(
                    Icons.widgets_outlined,
                    size: 14,
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
                const SizedBox(width: 6),
                // Wraps instead of truncating: a clipped path is unreadable and
                // uncopyable, and the panel scrolls anyway.
                Expanded(
                  child: Text(heading, style: theme.textTheme.bodySmall),
                ),
              ],
            ),
            if (showTargetsNote)
              Padding(
                padding: const EdgeInsets.only(left: 20, top: 4),
                child: Text(
                  footprintTargetsLabel(l10n, component.coverage),
                  key: ValueKey('component-footprint-coverage-$index'),
                  style: muted,
                ),
              ),
            if (shown.isNotEmpty)
              Padding(
                padding: const EdgeInsets.only(left: 20, top: 2),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    for (final t in shown) Text(t, style: muted),
                    if (extra > 0)
                      Text(
                        l10n.targetsMore(extra),
                        style: muted?.copyWith(fontStyle: FontStyle.italic),
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
