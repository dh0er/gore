import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart';

import '../../l10n/app_localizations.dart';
import '../../status/domain/status_notifier.dart';
import '../domain/conflicts_provider.dart';
import '../domain/library_notifier.dart';
import '../domain/models.dart';
import 'mod_labels.dart';

/// The id of the mod selected in the list, shown in the detail panel; null
/// when nothing is selected.
final selectedModProvider = StateProvider<String?>((ref) => null);

/// The reorderable list of mods in loadout order: each row toggles enablement,
/// shows the kind + component chips + a conflict badge, and selects the mod on
/// tap. Dragging a row calls [LibraryNotifier.reorder].
class ModList extends ConsumerWidget {
  const ModList({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final library = ref.watch(libraryProvider);
    final selected = ref.watch(selectedModProvider);
    final conflicts = ref.watch(conflictsProvider);
    final status = ref.watch(statusProvider);
    final mutationsBlocked =
        library.busy ||
        !library.authoritative ||
        status.busy ||
        conflicts.isLoading;

    // Join loadout order -> library mods; skip any entry whose mod is missing
    // (reconciliation should prevent this, but stay defensive).
    final rows = <({LoadoutEntryView entry, ModEntryMetaView mod})>[
      for (final entry in library.loadout.entries)
        if (library.modById(entry.id) case final mod?) (entry: entry, mod: mod),
    ];

    if (rows.isEmpty) {
      return Center(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Text(
            l10n.actionImport,
            style: Theme.of(context).textTheme.bodyMedium,
          ),
        ),
      );
    }

    final conflictList = conflicts.value ?? const <ConflictView>[];
    Widget itemBuilder(BuildContext context, int index) {
      final row = rows[index];
      final mod = row.mod;
      final summary = ConflictSummary.forMod(conflictList, mod.id);
      return _ModTile(
        key: ValueKey(mod.id),
        index: index,
        mod: mod,
        enabled: row.entry.enabled,
        mutationsBlocked: mutationsBlocked,
        selected: mod.id == selected,
        summary: summary,
        onTap: () => ref.read(selectedModProvider.notifier).state = mod.id,
        onToggle: () {
          final current = ref.read(libraryProvider);
          if (current.busy ||
              !current.authoritative ||
              ref.read(statusProvider).busy ||
              ref.read(conflictsProvider).isLoading) {
            return;
          }
          ref.read(libraryProvider.notifier).toggle(mod.id);
        },
      );
    }

    // SliverReorderableList adds semantic move actions independently of the
    // drag listener. Render a plain list while mutations are blocked so screen
    // readers are not offered actions that cannot run.
    if (mutationsBlocked) {
      return ListView.builder(itemCount: rows.length, itemBuilder: itemBuilder);
    }

    return ReorderableListView.builder(
      buildDefaultDragHandles: false,
      itemCount: rows.length,
      // onReorderItem hands us a newIndex that already treats the dragged item
      // as removed. LibraryNotifier.reorder uses the classic ReorderableList
      // convention (it subtracts 1 for downward moves itself), so translate
      // back to that convention here.
      onReorderItem: (oldIndex, newIndex) {
        final current = ref.read(libraryProvider);
        if (current.busy ||
            !current.authoritative ||
            ref.read(statusProvider).busy ||
            ref.read(conflictsProvider).isLoading) {
          return;
        }
        final classic = newIndex >= oldIndex ? newIndex + 1 : newIndex;
        ref.read(libraryProvider.notifier).reorder(oldIndex, classic);
      },
      itemBuilder: itemBuilder,
    );
  }
}

class _ModTile extends StatelessWidget {
  const _ModTile({
    super.key,
    required this.index,
    required this.mod,
    required this.enabled,
    required this.mutationsBlocked,
    required this.selected,
    required this.summary,
    required this.onTap,
    required this.onToggle,
  });

  final int index;
  final ModEntryMetaView mod;
  final bool enabled;
  final bool mutationsBlocked;
  final bool selected;
  final ConflictSummary summary;
  final VoidCallback onTap;
  final VoidCallback onToggle;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final subtitleParts = <String>[
      if (mod.version != null) 'v${mod.version}',
      if (mod.author != null) mod.author!,
    ];
    final chips = componentChips(mod.components);

    return Material(
      color: selected ? scheme.secondaryContainer : Colors.transparent,
      child: InkWell(
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.center,
            children: [
              Checkbox(
                value: enabled,
                onChanged: mutationsBlocked ? null : (_) => onToggle(),
              ),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Row(
                      children: [
                        Flexible(
                          child: Text(
                            mod.name.isEmpty ? mod.id : mod.name,
                            style: theme.textTheme.titleSmall?.copyWith(
                              color: enabled ? null : scheme.onSurfaceVariant,
                            ),
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                        const SizedBox(width: 6),
                        _KindChip(label: kindLabel(l10n, mod.kind)),
                      ],
                    ),
                    if (subtitleParts.isNotEmpty)
                      Padding(
                        padding: const EdgeInsets.only(top: 2),
                        child: Text(
                          subtitleParts.join(' · '),
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: scheme.onSurfaceVariant,
                          ),
                        ),
                      ),
                    if (chips.isNotEmpty)
                      Padding(
                        padding: const EdgeInsets.only(top: 4),
                        child: Wrap(
                          spacing: 4,
                          runSpacing: 2,
                          children: [
                            for (final chip in chips)
                              _ComponentChipWidget(label: chip.label),
                          ],
                        ),
                      ),
                    if (!enabled)
                      Padding(
                        padding: const EdgeInsets.only(top: 2),
                        child: Text(
                          l10n.modDisabledHint,
                          style: theme.textTheme.labelSmall?.copyWith(
                            color: scheme.onSurfaceVariant,
                          ),
                        ),
                      ),
                  ],
                ),
              ),
              const SizedBox(width: 4),
              ConflictBadge(summary: summary),
              ReorderableDragStartListener(
                index: index,
                enabled: !mutationsBlocked,
                child: Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 4),
                  child: Icon(
                    Icons.drag_handle,
                    color: scheme.onSurfaceVariant,
                    size: 20,
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// Small badge summarizing a mod's conflicts: red count when any are hard,
/// amber when only soft, grey when only info. Nothing when the mod is clean.
class ConflictBadge extends StatelessWidget {
  const ConflictBadge({super.key, required this.summary});

  final ConflictSummary summary;

  @override
  Widget build(BuildContext context) {
    if (summary.isEmpty) return const SizedBox.shrink();
    final scheme = Theme.of(context).colorScheme;
    final (Color color, int count) = summary.hard > 0
        ? (scheme.error, summary.hard)
        : summary.soft > 0
        ? (Colors.amber.shade700, summary.soft)
        : (scheme.onSurfaceVariant, summary.info);
    return Tooltip(
      message: 'H${summary.hard} S${summary.soft} I${summary.info}',
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
        decoration: BoxDecoration(
          color: color.withValues(alpha: 0.15),
          borderRadius: BorderRadius.circular(10),
          border: Border.all(color: color),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.warning_amber_rounded, size: 12, color: color),
            const SizedBox(width: 2),
            Text(
              '$count',
              style: TextStyle(
                color: color,
                fontSize: 11,
                fontWeight: FontWeight.w600,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _KindChip extends StatelessWidget {
  const _KindChip({required this.label});
  final String label;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 1),
      decoration: BoxDecoration(
        color: scheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(4),
      ),
      child: Text(
        label,
        style: TextStyle(fontSize: 11, color: scheme.onSurfaceVariant),
      ),
    );
  }
}

class _ComponentChipWidget extends StatelessWidget {
  const _ComponentChipWidget({required this.label});
  final String label;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 1),
      decoration: BoxDecoration(
        color: scheme.primaryContainer,
        borderRadius: BorderRadius.circular(4),
      ),
      child: Text(
        label,
        style: TextStyle(fontSize: 11, color: scheme.onPrimaryContainer),
      ),
    );
  }
}
