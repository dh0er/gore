import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/editor_notifier.dart';
import 'package:goresave/features/editor/domain/pending_edits.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/providers/data_providers.dart';

/// Whether this save carries the damage the repair addresses.
///
/// Nothing but the count: the warning says the game will act on the wrong item,
/// which a reader needs to know even where nothing can be repaired — including
/// a save whose core offers no repair at all.
bool slotRepairWarranted(SaveInspection inspection) =>
    inspection.privateInventory.misalignedSlots > 0;

/// Why the repair can or cannot be queued. The two obstacles read differently to
/// a user — one is about this save's write capability, the other about the
/// repair not being on offer at all — so they are kept apart rather than
/// collapsed into one "unavailable".
enum SlotRepairAvailability {
  /// Offered and writable: the action can be queued.
  available,

  /// The core does not advertise the repair for this save, while other edits
  /// may well be fine.
  notOffered,

  /// Offered, but this save cannot take a private write right now.
  notWritable,
}

/// Which of the above applies. Repairing is a private write, so it takes the
/// same capability every other inventory action is gated on; without it — or
/// without the core offering the op — the action could be queued but never
/// applied.
SlotRepairAvailability slotRepairAvailability(
  SaveInspection inspection, {
  required bool canCompress,
}) {
  if (!inspection.privateInventory.canRepairSlots) {
    return SlotRepairAvailability.notOffered;
  }
  if (!inspection.privateEditable ||
      !inspection.privateTypedVerified ||
      !canCompress) {
    return SlotRepairAvailability.notWritable;
  }
  return SlotRepairAvailability.available;
}

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
    required this.availability,
  });

  final EditorNotifier notifier;

  /// How many slots across the whole save are affected.
  final int misalignedSlots;

  /// Whether the repair can be queued, and if not, why. The warning shows
  /// either way — the save is damaged regardless — but anything other than
  /// [SlotRepairAvailability.available] replaces the action with its reason.
  final SlotRepairAvailability availability;

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
                  if (availability != SlotRepairAvailability.available) ...[
                    const SizedBox(height: 4),
                    Text(
                      availability == SlotRepairAvailability.notOffered
                          ? l10n.slotRepairNotOffered
                          : l10n.slotRepairUnavailable,
                      style: theme.textTheme.bodyMedium?.copyWith(
                        color: scheme.onErrorContainer,
                        fontStyle: FontStyle.italic,
                      ),
                    ),
                  ],
                ],
              ),
            ),
            if (availability == SlotRepairAvailability.available) ...[
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
                  // No confirmation step: the button only QUEUES the repair, the
                  // banner already states what it does, and Discard takes it
                  // back before anything reaches the save.
                  onPressed: () => notifier.setPendingEdit(
                    pendingKey,
                    const PendingSaveEdit(
                      edits: [
                        {
                          'path': 'private.inventory.repairSlots',
                          'value': <String, Object?>{},
                        },
                      ],
                    ),
                  ),
                ),
            ],
          ],
        ),
      ),
    );
  }
}
