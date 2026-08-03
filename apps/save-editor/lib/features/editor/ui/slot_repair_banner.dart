import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/editor_notifier.dart';
import 'package:goresave/features/editor/domain/pending_edits.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/providers/data_providers.dart';

/// Whether this save both needs the slot repair and can actually receive it.
///
/// Repairing is a private write, so it takes the same capability every other
/// inventory action is gated on — otherwise the action could be queued but never
/// applied. Shared by the places that surface the warning (the overview and the
/// inventory) so they can never disagree about it.
bool slotRepairAvailable(
  SaveInspection inspection, {
  required bool canCompress,
}) =>
    inspection.privateInventory.canRepairSlots &&
    inspection.privateEditable &&
    inspection.privateTypedVerified &&
    canCompress;

/// Warns that this savegame carries inventory slots whose stored id no longer
/// matches the position they sit in, and offers the repair.
///
/// The game addresses an inventory slot by its position; older versions of this
/// editor inserted and deleted slots in a way that moved items away from the id
/// that identifies them. In the game that shows up as dropping one item while a
/// different one disappears. The repair rewrites the ids to match the positions
/// — the shape the game itself writes — and changes nothing else.
///
/// Repairing is a normal queued edit: it lands in the save only when the user
/// saves, with the usual backup.
class SlotRepairBanner extends ConsumerWidget {
  const SlotRepairBanner({
    super.key,
    required this.notifier,
    required this.misalignedSlots,
  });

  final EditorNotifier notifier;

  /// How many slots across the whole save are affected.
  final int misalignedSlots;

  /// Pending-edit registry key. One per save; the repair is whole-save.
  static const String pendingKey = 'inventoryRepair';

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    // Rebuild when the queued state changes (queue / cancel / saved).
    final queued = ref
        .watch(editorProvider)
        .pendingEdits
        .containsKey(pendingKey);

    final scheme = theme.colorScheme;
    return Card(
      margin: EdgeInsets.zero,
      color: scheme.errorContainer,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Icon(Icons.report_problem_outlined, color: scheme.onErrorContainer),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    l10n.slotRepairTitle,
                    style: theme.textTheme.titleSmall?.copyWith(
                      color: scheme.onErrorContainer,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  const SizedBox(height: 4),
                  Text(
                    queued
                        ? l10n.slotRepairQueued
                        : l10n.slotRepairBody(misalignedSlots),
                    style: theme.textTheme.bodyMedium?.copyWith(
                      color: scheme.onErrorContainer,
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(width: 12),
            if (queued)
              TextButton(
                onPressed: () => notifier.clearPendingEdit(pendingKey),
                child: Text(l10n.slotRepairDiscard),
              )
            else
              FilledButton.icon(
                icon: const Icon(Icons.healing_outlined, size: 18),
                label: Text(l10n.slotRepairAction),
                onPressed: () => _confirmAndQueue(context, l10n),
              ),
          ],
        ),
      ),
    );
  }

  Future<void> _confirmAndQueue(
    BuildContext context,
    AppLocalizations l10n,
  ) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: Text(l10n.slotRepairTitle),
        content: Text(l10n.slotRepairConfirm(misalignedSlots)),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(dialogContext).pop(false),
            child: Text(l10n.cancel),
          ),
          FilledButton(
            onPressed: () => Navigator.of(dialogContext).pop(true),
            child: Text(l10n.slotRepairAction),
          ),
        ],
      ),
    );
    if (confirmed != true) return;
    notifier.setPendingEdit(
      pendingKey,
      const PendingSaveEdit(
        edits: [
          {
            'path': 'private.inventory.repairSlots',
            'value': <String, Object?>{},
          },
        ],
      ),
    );
  }
}
