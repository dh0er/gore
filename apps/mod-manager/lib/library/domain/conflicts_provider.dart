import 'dart:convert';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/providers.dart';
import 'library_notifier.dart';
import 'models.dart';

/// Conflicts across the enabled mods of the current loadout.
///
/// Re-runs when the loadout's ENTRIES change (toggle/reorder) OR when a mod's
/// CONTENT changes (a same-id re-import that swaps its components/targets),
/// so analyze recomputes automatically in both cases.
final conflictsProvider = FutureProvider<List<ConflictView>>((ref) {
  // Key on a stable String derived from BOTH:
  //  * the loadout entries (id + enabled) — toggling/reordering changes this;
  //  * each mod's content signature (id + every analyzer-relevant component fact) —
  //    re-importing a mod under the SAME id changes LibraryState.mods (its
  //    components/targets) but NOT the loadout entries, so without this a
  //    same-id update would keep the stale conflicts.
  // NOT the whole LibraryState: a mutation flips LibraryState.busy at its start
  // while it persists the new loadout via mgr_set_loadout asynchronously.
  // Re-analyzing on that busy flip would read the still-stale on-disk loadout.
  // Both `loadout` and `mods` only change on a settled refresh (after the
  // persist lands / the library reload completes), not on the busy flip, so
  // this preserves the "don't recompute mid-mutation" property.
  final libraryKey = ref.watch(
    libraryProvider.select((s) => (s.authoritative, _conflictsKey(s))),
  );
  if (!libraryKey.$1) return const [];
  return ref.watch(mgrFfiProvider).analyze();
});

/// Whether enabled components make a conflict result incomplete by contract.
/// This display qualifier is not an Apply gate or runtime-priority claim.
final enabledFootprintKnowledgeIncompleteProvider = Provider<bool>((ref) {
  return hasIncompleteEnabledFootprintKnowledge(ref.watch(libraryProvider));
});

/// Join exact, case-sensitive loadout ids to library metadata. Disabled mods do
/// not qualify the current analysis, while a missing enabled id fails closed.
bool hasIncompleteEnabledFootprintKnowledge(LibraryState state) {
  if (!state.authoritative) return false; // The caller shows "unverified".
  final modsById = <String, ModEntryMetaView>{};
  for (final mod in state.mods) {
    // Preserve LibraryState.modById's defensive first-match behavior.
    modsById.putIfAbsent(mod.id, () => mod);
  }
  for (final entry in state.loadout.entries) {
    if (!entry.enabled) continue;
    final mod = modsById[entry.id];
    if (mod == null) return true;
    if (mod.components.any(
      (component) => component.coverage != FootprintCoverage.exact,
    )) {
      return true;
    }
  }
  return false;
}

/// A stable, value-comparable key that changes when the loadout entries change
/// OR when any mod's deployable content (kind, coverage, opacity, targets, or
/// raw-file destination) changes — including a same-id re-import.
String _conflictsKey(LibraryState s) {
  // JSON gives every list and string an unambiguous boundary. Hand-built
  // delimiters alias valid values (`['a+b']` versus `['a', 'b']`) and can keep
  // a stale analysis cached after a same-id re-import.
  return jsonEncode({
    'loadout': [
      for (final entry in s.loadout.entries)
        {'id': entry.id, 'enabled': entry.enabled},
    ],
    'mods': [
      for (final mod in s.mods)
        {
          'id': mod.id,
          'components': [
            for (final component in mod.components)
              {
                // `raw` preserves fields this app version does not model. They
                // can still affect a newer native analyzer, so reconstructing
                // only today's view would leave stale conflicts cached.
                'raw': _canonicalJson(component.raw),
                // Missing/unknown wire values are interpreted conservatively
                // by ComponentView; cache that effective grade as well.
                'coverage': component.coverage.name,
              },
          ],
        },
    ],
  });
}

/// Recursively sort JSON object keys while preserving array order and scalar
/// types. FFI-decoded component [raw] values are JSON by contract, so this is
/// a stable semantic encoding rather than one that depends on map insertion
/// order.
Object? _canonicalJson(Object? value) {
  if (value is List) {
    return [for (final item in value) _canonicalJson(item)];
  }
  if (value is Map) {
    final entries = [
      for (final entry in value.entries)
        MapEntry(entry.key as String, _canonicalJson(entry.value)),
    ]..sort((a, b) => a.key.compareTo(b.key));
    return <String, Object?>{
      for (final entry in entries) entry.key: entry.value,
    };
  }
  return value;
}

/// Every conflict that involves [modId].
List<ConflictView> conflictsForMod(List<ConflictView> conflicts, String modId) {
  return [
    for (final c in conflicts)
      if (c.modIds.contains(modId)) c,
  ];
}

/// A conflict's mod ids arranged in loadout order (lowest priority first),
/// with the winner flagged for proven soft/hard conflicts. Informational
/// unknown-footprint advisories have no winner. Load order list order is
/// priority-ascending on the Rust side, so the highest-priority (last)
/// participant wins otherwise. Ids not present in [order] keep their original
/// relative position at the front. When the conflict names fewer than two mods,
/// none is marked a winner.
class OrderedConflictChain {
  const OrderedConflictChain(this.modIds, this.winnerId);

  /// The conflict's mods, lowest-priority first.
  final List<String> modIds;

  /// The winning (highest-priority) mod id, or null for info advisories/no contest.
  final String? winnerId;

  bool isWinner(String modId) => modId == winnerId;
}

/// Order [conflict]'s participants by the loadout [order] (a list of mod ids,
/// lowest priority first) and pick the winner (highest priority present) for a
/// proven soft/hard conflict. `info` advisories intentionally have no winner.
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
  final winner = conflict.severity != 'info' && ids.length >= 2
      ? ids.last
      : null;
  return OrderedConflictChain(ids, winner);
}

/// Per-mod tally of conflicts by severity.
class ConflictSummary {
  const ConflictSummary({this.hard = 0, this.soft = 0, this.info = 0});

  /// Count the conflicts touching [modId], bucketed by severity. Unknown
  /// severities are ignored.
  factory ConflictSummary.forMod(List<ConflictView> conflicts, String modId) {
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
