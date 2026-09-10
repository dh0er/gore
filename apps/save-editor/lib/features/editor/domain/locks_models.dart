/// Result of `private.locks.list`.
///
/// A save records only which locks the player has ALREADY opened, so this is
/// the smaller half of the picture: the panel joins it against the bundled
/// lock catalog, which is what knows the other 200-odd locks that are still
/// shut. Carries an optional [error] (set by the notifier instead of throwing)
/// so the panel renders failures inline.
class LocksResult {
  LocksResult({this.unlocked = const [], this.writable = const [], this.error})
    : _folded = {for (final name in unlocked) name.toLowerCase()};

  factory LocksResult.fromJson(Map<String, Object?> json) {
    return LocksResult(
      unlocked: (json['unlocked'] as List? ?? const [])
          .whereType<String>()
          .toList(growable: false),
      writable: (json['writable'] as List? ?? const [])
          .whereType<String>()
          .toList(growable: false),
    );
  }

  /// Every lock the player has opened, spelled as the save spells it. The
  /// spelling matters for a name the catalog does not know: that row is
  /// rendered and edited under the save's own name, not a folded copy.
  final List<String> unlocked;

  /// Lower-cased [unlocked], for membership tests. The save's set is an FName
  /// set, so a catalog entry differing only in case is the same lock.
  final Set<String> _folded;

  /// Ops the core will accept for this save. Empty when the save carries no
  /// editable lock set, which is what keeps the panel read-only instead of
  /// offering a toggle the write path would refuse.
  final List<String> writable;

  final String? error;

  bool get canSetUnlocked => writable.contains('private.locks.setUnlocked');

  bool isUnlocked(String name) => _folded.contains(name.toLowerCase());
}

/// Pending lock change → `private.locks.setUnlocked`. Declarative: the state
/// one lock shall be in. Keyed by the lock name in the pending map, so
/// toggling a row back to its saved value drops the edit instead of stacking a
/// second one.
class LockSetUnlockedEdit {
  const LockSetUnlockedEdit({required this.lock, required this.unlocked});

  final String lock;
  final bool unlocked;

  Map<String, Object?> toEditJson() {
    return {
      'path': 'private.locks.setUnlocked',
      'value': {'lock': lock, 'unlocked': unlocked},
    };
  }
}
