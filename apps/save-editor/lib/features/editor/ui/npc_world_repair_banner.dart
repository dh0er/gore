import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/editor_notifier.dart';
import 'package:goresave/features/editor/ui/slot_repair_banner.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/providers/data_providers.dart';

/// Whether this save holds NPCs an older build revived into nowhere.
bool npcWorldRepairWarranted(SaveInspection inspection) =>
    inspection.privateNpc.strandedRevives.isNotEmpty;

/// Whether the repair can be queued, and if not, why. It is a revive per NPC,
/// so it takes the capability every revive is gated on.
RepairAvailability npcWorldRepairAvailability(
  SaveInspection inspection, {
  required bool canCompress,
}) {
  if (!inspection.privateNpc.canRevive) {
    return RepairAvailability.notOffered;
  }
  if (!inspection.privateEditable ||
      !inspection.privateTypedVerified ||
      !canCompress) {
    return RepairAvailability.notWritable;
  }
  return RepairAvailability.available;
}

/// Warns that older versions of this editor revived NPCs the game had already
/// taken out of the world, and offers the repair.
///
/// Those versions cleared every death marker but left the NPC in the game's
/// removed-from-world list, so it counts as alive yet never spawns. Reviving it
/// again takes it off that list — everything else the old revive already did.
///
/// The repair queues one ordinary revive per NPC, under the same pending keys
/// the NPC panel uses, so it lands only when the user saves, with the usual
/// backup, and shows as a pending revive on each NPC.
class NpcWorldRepairBanner extends ConsumerWidget {
  const NpcWorldRepairBanner({
    super.key,
    required this.notifier,
    required this.npcIds,
    required this.availability,
  });

  final EditorNotifier notifier;

  /// The NPCs to put back into the world.
  final List<String> npcIds;

  /// See [SlotRepairBanner.availability].
  final RepairAvailability availability;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    // Rebuild when the queued state changes (queue / cancel / saved).
    final pending = ref.watch(editorProvider).pendingEdits;
    final queued = npcIds.every(
      (id) => pending.containsKey(EditorNotifier.npcRevivePendingKey(id)),
    );

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
                    l10n.npcWorldRepairTitle,
                    style: theme.textTheme.titleSmall?.copyWith(
                      color: scheme.onErrorContainer,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  const SizedBox(height: 4),
                  Text(
                    queued
                        ? l10n.slotRepairQueued
                        : l10n.npcWorldRepairBody(npcIds.length),
                    style: theme.textTheme.bodyMedium?.copyWith(
                      color: scheme.onErrorContainer,
                    ),
                  ),
                  if (availability != RepairAvailability.available) ...[
                    const SizedBox(height: 4),
                    Text(
                      availability == RepairAvailability.notOffered
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
            if (availability == RepairAvailability.available) ...[
              const SizedBox(width: 12),
              if (queued)
                TextButton(
                  onPressed: () {
                    for (final id in npcIds) {
                      notifier.clearPendingEdit(
                        EditorNotifier.npcRevivePendingKey(id),
                      );
                    }
                  },
                  child: Text(l10n.slotRepairDiscard),
                )
              else
                FilledButton.icon(
                  icon: const Icon(Icons.healing_outlined, size: 18),
                  label: Text(l10n.slotRepairAction),
                  // No confirmation step, as for the slot repair: the button
                  // only QUEUES the revives and Discard takes them back.
                  onPressed: () {
                    for (final id in npcIds) {
                      notifier.setPendingNpcRevive(id);
                    }
                  },
                ),
            ],
          ],
        ),
      ),
    );
  }
}
