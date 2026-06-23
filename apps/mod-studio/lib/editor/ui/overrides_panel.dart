import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path/path.dart' as p;
import '../../audio/domain/audio_replacements_notifier.dart';
import '../../l10n/app_localizations.dart';
import '../../loc/domain/loc_edits_notifier.dart';
import '../domain/override_entry.dart';
import '../domain/overrides_notifier.dart';

/// Unified "Changes" panel: lists every staged mod change across the three
/// domains (item value overrides, localized text edits, audio replacements),
/// each row individually removable, with a single clear-all action.
class OverridesPanel extends ConsumerWidget {
  const OverridesPanel({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final overridesState = ref.watch(overridesProvider);
    final overrides      = ref.read(overridesProvider.notifier);
    final locState       = ref.watch(locEditsProvider);
    final locEdits       = ref.read(locEditsProvider.notifier);
    final audioState     = ref.watch(audioReplacementsProvider);
    final audio          = ref.read(audioReplacementsProvider.notifier);

    final theme  = Theme.of(context);
    final scheme = theme.colorScheme;
    final l10n   = AppLocalizations.of(context);

    final overrideEntries = overridesState.entries;
    final locPairs        = <_LocEditRow>[
      for (final outer in locState.edits.entries)
        for (final inner in outer.value.entries)
          _LocEditRow(locId: outer.key, set: inner.key, text: inner.value),
    ];
    final audioEntries = audioState.entries;

    final total   = overridesState.count + locState.entryCount + audioState.count;
    final isEmpty = total == 0;

    return Column(
      children: [
        Container(
          color: scheme.surfaceContainerLowest,
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
          child: Row(
            children: [
              const Icon(Icons.pending_actions_outlined),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  'Changes ($total)',
                  style: theme.textTheme.titleSmall,
                ),
              ),
              if (!isEmpty)
                TextButton.icon(
                  icon: const Icon(Icons.clear_all, size: 18),
                  label: Text(l10n.clearAll),
                  onPressed: () {
                    overrides.clearAll();
                    locEdits.clearAll();
                    audio.clearAll();
                  },
                ),
            ],
          ),
        ),
        const Divider(height: 1),
        Expanded(
          child: isEmpty
              ? Center(
                  child: Text(
                    l10n.noPendingOverrides,
                    textAlign: TextAlign.center,
                    style: TextStyle(color: scheme.onSurfaceVariant),
                  ),
                )
              : ListView(
                  padding: const EdgeInsets.symmetric(vertical: 6),
                  children: [
                    if (overrideEntries.isNotEmpty) ...[
                      const _SectionHeader('Item values'),
                      for (final entry in overrideEntries)
                        _OverrideRow(entry: entry, notifier: overrides),
                    ],
                    if (locPairs.isNotEmpty) ...[
                      const _SectionHeader('Localized text'),
                      for (final row in locPairs)
                        _LocRow(row: row, notifier: locEdits),
                    ],
                    if (audioEntries.isNotEmpty) ...[
                      const _SectionHeader('Audio'),
                      for (final entry in audioEntries)
                        _AudioRow(entry: entry, notifier: audio),
                    ],
                  ],
                ),
        ),
      ],
    );
  }
}

class _SectionHeader extends StatelessWidget {
  const _SectionHeader(this.title);

  final String title;

  @override
  Widget build(BuildContext context) {
    final theme  = Theme.of(context);
    final scheme = theme.colorScheme;
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 12, 16, 4),
      child: Text(
        title,
        style: theme.textTheme.labelSmall?.copyWith(
          color: scheme.onSurfaceVariant,
          fontWeight: FontWeight.w700,
          letterSpacing: 0.6,
        ),
      ),
    );
  }
}

class _OverrideRow extends StatelessWidget {
  const _OverrideRow({required this.entry, required this.notifier});

  final OverrideEntry entry;
  final OverridesNotifier notifier;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
      child: Row(
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  '${entry.classId}.${entry.field}',
                  style: const TextStyle(fontFamily: 'Consolas', fontSize: 12),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
                Text(
                  '${entry.oldValue} → ${entry.newValue}',
                  style: TextStyle(
                    fontSize: 12,
                    color: scheme.primary,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ],
            ),
          ),
          IconButton(
            icon: const Icon(Icons.remove_circle_outline, size: 18),
            tooltip: AppLocalizations.of(context).removeOverride,
            onPressed: () => notifier.removeOverride(entry.key),
          ),
        ],
      ),
    );
  }
}

class _LocEditRow {
  const _LocEditRow({required this.locId, required this.set, required this.text});

  final String locId;
  final String set;
  final String text;
}

class _LocRow extends StatelessWidget {
  const _LocRow({required this.row, required this.notifier});

  final _LocEditRow row;
  final LocEditsNotifier notifier;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
      child: Row(
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  '${row.locId}  ·  ${row.set}',
                  style: const TextStyle(fontFamily: 'Consolas', fontSize: 12),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
                Text(
                  row.text,
                  style: TextStyle(
                    fontSize: 12,
                    color: scheme.primary,
                    fontWeight: FontWeight.w600,
                  ),
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                ),
              ],
            ),
          ),
          IconButton(
            icon: const Icon(Icons.remove_circle_outline, size: 18),
            tooltip: AppLocalizations.of(context).removeOverride,
            onPressed: () => notifier.removeEdit(row.locId, row.set),
          ),
        ],
      ),
    );
  }
}

class _AudioRow extends StatelessWidget {
  const _AudioRow({required this.entry, required this.notifier});

  final AudioReplacement entry;
  final AudioReplacementsNotifier notifier;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
      child: Row(
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  '${entry.bank}  ·  ${entry.sample}',
                  style: const TextStyle(fontFamily: 'Consolas', fontSize: 12),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
                Text(
                  p.basename(entry.wavPath),
                  style: TextStyle(
                    fontSize: 12,
                    color: scheme.primary,
                    fontWeight: FontWeight.w600,
                  ),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ],
            ),
          ),
          IconButton(
            icon: const Icon(Icons.remove_circle_outline, size: 18),
            tooltip: AppLocalizations.of(context).removeOverride,
            onPressed: () => notifier.remove(entry.key),
          ),
        ],
      ),
    );
  }
}
