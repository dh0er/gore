import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../l10n/app_localizations.dart';
import '../domain/override_entry.dart';
import '../domain/overrides_notifier.dart';

/// Shows all pending overrides with per-row remove and a clear-all action.
class OverridesPanel extends ConsumerWidget {
  const OverridesPanel({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final state    = ref.watch(overridesProvider);
    final notifier = ref.read(overridesProvider.notifier);
    final entries  = state.entries;
    final theme    = Theme.of(context);
    final scheme   = theme.colorScheme;
    final l10n     = AppLocalizations.of(context);

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
                  l10n.pendingOverridesWithCount(state.count),
                  style: theme.textTheme.titleSmall,
                ),
              ),
              if (entries.isNotEmpty)
                TextButton.icon(
                  icon: const Icon(Icons.clear_all, size: 18),
                  label: Text(l10n.clearAll),
                  onPressed: notifier.clearAll,
                ),
            ],
          ),
        ),
        const Divider(height: 1),
        Expanded(
          child: entries.isEmpty
              ? Center(
                  child: Text(
                    l10n.noPendingOverrides,
                    textAlign: TextAlign.center,
                    style: TextStyle(color: scheme.onSurfaceVariant),
                  ),
                )
              : ListView.separated(
                  padding: const EdgeInsets.symmetric(vertical: 6),
                  itemCount: entries.length,
                  separatorBuilder: (context, index) => const Divider(height: 1, indent: 16),
                  itemBuilder: (context, index) {
                    final entry = entries[index];
                    return _OverrideRow(entry: entry, notifier: notifier);
                  },
                ),
        ),
      ],
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
