/// A single NPC actor row from the core `private.npc.list` command. Carries the
/// id (a GlobalId used to build an [Actor.npc]) plus a small status snapshot
/// (dead / hp), e.g. driving the NPC status row's revive action.
class NpcActor {
  const NpcActor({
    required this.id,
    required this.isDead,
    this.hp,
    this.maxHp,
  });

  factory NpcActor.fromJson(Map<String, Object?> json) {
    return NpcActor(
      id: json['id'] as String? ?? '',
      isDead: json['isDead'] == true,
      hp: (json['hp'] as num?)?.toDouble(),
      maxHp: (json['maxHp'] as num?)?.toDouble(),
    );
  }

  /// GlobalId for this NPC (e.g. `Lizard-WP_EF_...`).
  final String id;

  /// True ONLY when the NPC was KILLED. A merely defeated / knocked-out NPC is
  /// still ALIVE (`isDead == false`). Same killed-only semantics as the
  /// master list's skull avatar; here it drives the NPC status row and the
  /// Revive action's enablement.
  final bool isDead;
  final double? hp;
  final double? maxHp;
}

/// A paginated page of NPC actors, mirroring the shape of the other progression
/// page models (e.g. `MemoryCharactersPage`): the typed rows plus the
/// server-side pagination cursor and an optional inline [error]. The core caps
/// `total` at the full NPC count (~1484) so the list must be paginated.
class NpcActorsPage {
  const NpcActorsPage({
    this.npcs = const [],
    this.total = 0,
    this.offset = 0,
    this.limit = 100,
    this.error,
  });

  factory NpcActorsPage.fromJson(Map<String, Object?> json) {
    return NpcActorsPage(
      npcs:
          (json['npcs'] as List?)
              ?.whereType<Map>()
              .map((e) => NpcActor.fromJson(e.cast<String, Object?>()))
              .toList(growable: false) ??
          const [],
      total: (json['total'] as num?)?.toInt() ?? 0,
      offset: (json['offset'] as num?)?.toInt() ?? 0,
      limit: (json['limit'] as num?)?.toInt() ?? 100,
    );
  }

  final List<NpcActor> npcs;
  final int total;
  final int offset;
  final int limit;
  final String? error;

  bool get hasNext => offset + npcs.length < total;
  bool get hasPrevious => offset > 0;
  int get pageIndex => limit == 0 ? 0 : offset ~/ limit;
  int get pageCount => total == 0 ? 1 : (total + limit - 1) ~/ limit;
}
