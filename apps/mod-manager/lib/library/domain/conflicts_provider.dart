import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/providers.dart';
import 'library_notifier.dart';
import 'models.dart';

/// Conflicts across the enabled mods of the current loadout.
///
/// Re-runs when the loadout's ENTRIES change, so toggling or reordering a mod
/// re-analyzes automatically.
final conflictsProvider = FutureProvider<List<ConflictView>>((ref) {
  // Key on the entries (id + enabled), NOT the whole LibraryState: a mutation
  // flips LibraryState.busy at its start while it persists the new loadout via
  // mgr_set_loadout asynchronously. Re-analyzing on that busy flip would read
  // the still-stale on-disk loadout. The notifier only updates state.loadout
  // AFTER the persist lands, so keying on the entries defers the analyze to a
  // point where mgr_analyze reads the already-written loadout.
  ref.watch(libraryProvider.select(
    (s) => [for (final e in s.loadout.entries) '${e.id}:${e.enabled}'].join(','),
  ));
  return ref.watch(mgrFfiProvider).analyze();
});

/// Every conflict that involves [modId].
List<ConflictView> conflictsForMod(
  List<ConflictView> conflicts,
  String modId,
) {
  return [
    for (final c in conflicts)
      if (c.modIds.contains(modId)) c,
  ];
}

/// A conflict's mod ids arranged in loadout order (lowest priority first),
/// with the winner flagged. Load order list order is priority-ascending on the
/// Rust side, so the highest-priority (last) participant wins. Ids not present
/// in [order] keep their original relative position at the front. When the
/// conflict names fewer than two mods, none is marked a winner.
class OrderedConflictChain {
  const OrderedConflictChain(this.modIds, this.winnerId);

  /// The conflict's mods, lowest-priority first.
  final List<String> modIds;

  /// The winning (highest-priority) mod id, or null when there's no contest.
  final String? winnerId;

  bool isWinner(String modId) => modId == winnerId;
}

/// Order [conflict]'s participants by the loadout [order] (a list of mod ids,
/// lowest priority first) and pick the winner (highest priority present).
OrderedConflictChain orderConflictChain(
  ConflictView conflict,
  List<String> order,
) {
  final rank = <String, int>{
    for (var i = 0; i < order.length; i++) order[i]: i,
  };
  final ids = [...conflict.modIds];
  // Stable sort by loadout rank; unknown ids (rank absent) sort to the front.
  ids.sort((a, b) => (rank[a] ?? -1).compareTo(rank[b] ?? -1));
  final winner = ids.length >= 2 ? ids.last : null;
  return OrderedConflictChain(ids, winner);
}

/// Per-mod tally of conflicts by severity.
class ConflictSummary {
  const ConflictSummary({this.hard = 0, this.soft = 0, this.info = 0});

  /// Count the conflicts touching [modId], bucketed by severity. Unknown
  /// severities are ignored.
  factory ConflictSummary.forMod(
    List<ConflictView> conflicts,
    String modId,
  ) {
    var hard = 0;
    var soft = 0;
    var info = 0;
    for (final c in conflicts) {
      if (!c.modIds.contains(modId)) continue;
      switch (c.severity) {
        case 'hard':
          hard++;
        case 'soft':
          soft++;
        case 'info':
          info++;
      }
    }
    return ConflictSummary(hard: hard, soft: soft, info: info);
  }

  final int hard;
  final int soft;
  final int info;

  /// Total across all severities.
  int get total => hard + soft + info;

  /// True when nothing touches this mod.
  bool get isEmpty => total == 0;
}
