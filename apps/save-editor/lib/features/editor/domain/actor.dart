/// Whether a selected [Actor] is the player or a specific NPC.
enum ActorKind { player, npc }

/// A "selected actor" shared between the player/NPC-aware editor tabs (e.g.
/// the attribute and inventory tabs) so they can both operate on the player OR
/// a specific NPC, kept in sync via the editor state.
class Actor {
  /// The player. Has no [id]; [name] is always 'Player'. Its [uniqueName]
  /// (dialog-knowledge key) is always 'Hero'.
  const Actor.player()
    : kind = ActorKind.player,
      id = null,
      name = 'Player',
      uniqueName = 'Hero',
      isDead = false;

  /// A specific NPC, identified by its [id] (a GlobalId). [uniqueName] is the
  /// NPC's dialog-knowledge key (the GlobalId prefix before the first `-`, in
  /// original case). [isDead] is the NPC's known dead state at selection time,
  /// carried so the status row can show it (and keep Revive enabled) even when
  /// the async summary reload fails or the id is missing from the loaded page.
  const Actor.npc({
    required String this.id,
    required this.name,
    required this.uniqueName,
    this.isDead = false,
  }) : kind = ActorKind.npc;

  final ActorKind kind;

  /// GlobalId for NPCs, null for the player.
  final String? id;

  final String name;

  /// Dialog-knowledge key: the GlobalId prefix before the first `-` in original
  /// case for NPCs, 'Hero' for the player. A carried label, not identity —
  /// excluded from `==`/`hashCode` like [name] and [isDead].
  final String uniqueName;

  /// Known dead state (NPCs only; always false for the player). A label, not
  /// identity — excluded from `==`/`hashCode` like [name].
  final bool isDead;

  bool get isPlayer => kind == ActorKind.player;

  // Value equality by kind + id, so selection comparisons and provider rebuild
  // checks treat two Actors for the same target as equal. Name is intentionally
  // excluded — the id (or player-ness) is the identity; the name is a label.
  @override
  bool operator ==(Object other) =>
      other is Actor && other.kind == kind && other.id == id;

  @override
  int get hashCode => Object.hash(kind, id);

  @override
  String toString() => 'Actor(${kind.name}, id: $id, name: $name)';
}
