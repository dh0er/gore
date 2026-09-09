import 'dart:convert';

import 'package:flutter/services.dart' show rootBundle;

/// Whether a lock sits on a chest or on a door. The save does not distinguish
/// the two — both are names in the same set — so the split comes from the
/// catalog, where it is read off the game's own class defaults.
enum LockKind { chest, door }

/// How many of the game's four difficulty pips are filled for a raw 1..7 tier.
///
/// The lockpicking widget (`/Game/UI/LockPick/W_LockPickUI`) holds exactly four
/// `Difficulty_1..4` images and fills each one when the tier reaches its own
/// threshold. The thresholds are 1, 2, 4, and 6 — so the seven internal tiers
/// collapse onto four bars as 1 / 2,3 / 4,5 / 6,7, and the player never sees
/// the raw number anywhere in the game.
const lockDifficultyBarThresholds = [1, 2, 4, 6];

/// Total pips the game draws; the unfilled ones use its "empty" texture.
const lockDifficultyBarCount = 4;

int lockDifficultyBars(int difficulty) =>
    lockDifficultyBarThresholds.where((t) => difficulty >= t).length;

/// One lockable chest or door in the game.
///
/// [name] is the key the save is addressed by: the lock-bearing class's
/// `m_UniqueName`, which is exactly what turns up in
/// `LockPersistentData.m_UnlockedLocks` once the player has opened it. The
/// class's own `m_Lock` id ([lockId]) is a different string and never appears
/// in a save; it is carried only so a row can be traced back to the script.
class LockEntry {
  LockEntry({
    required this.name,
    required this.kind,
    required this.area,
    required this.difficulty,
    required this.lockId,
    required this.keys,
    required this.randomized,
  }) : search = [name, ...keys].join(' ').toLowerCase();

  final String name;
  final LockKind kind;

  /// Area code, matching a `LocationArea.id`, or `''` when unknown.
  final String area;

  /// Lockpicking difficulty, 1–7. Null for a lock that only ever opens with a
  /// key, and for the chests whose lock the randomizer assigns at runtime.
  final int? difficulty;

  /// The script-side lock id (`OC_Chest_Dexter_Lock`). Never a save key.
  final String? lockId;

  /// Item ids of the keys that open this lock, if any.
  final List<String> keys;

  /// Whether the randomized-lock subsystem registers this chest. Those carry no
  /// fixed difficulty of their own, but they do get a lock in a running game.
  final bool randomized;

  /// Lowercased [name] plus key ids, computed ONCE at parse time. The list is
  /// filtered client-side, so each keystroke must be a substring scan over
  /// cached strings rather than hundreds of fresh `toLowerCase()` calls.
  final String search;
}

/// The bundled catalog of every lock in the game, generated from the shipped
/// AngelScript cache by `apps/save-editor/tools/build_lock_catalog.py`.
/// Regenerate after a game patch: the lock set is cook-specific.
class LockCatalog {
  LockCatalog({required this.locks})
    : _byName = {for (final lock in locks) lock.name.toLowerCase(): lock};

  final List<LockEntry> locks;
  final Map<String, LockEntry> _byName;

  /// Case-insensitive, matching UE `FName` semantics: a save may spell a lock
  /// differently from the catalog and still mean the same lock.
  LockEntry? byName(String name) => _byName[name.toLowerCase()];

  static LockCatalog fromJsonString(String json) {
    final root = jsonDecode(json) as Map<String, Object?>;
    final locks = (root['locks'] as List? ?? const [])
        .whereType<Map<String, Object?>>()
        .map(
          (lock) => LockEntry(
            name: lock['n'] as String? ?? '',
            kind: lock['k'] == 'door' ? LockKind.door : LockKind.chest,
            area: lock['a'] as String? ?? '',
            difficulty: (lock['d'] as num?)?.toInt(),
            lockId: lock['l'] as String?,
            keys: (lock['keys'] as List? ?? const [])
                .whereType<String>()
                .toList(growable: false),
            randomized: lock['r'] == true,
          ),
        )
        .where((lock) => lock.name.isNotEmpty)
        .toList()
      ..sort((a, b) => a.name.compareTo(b.name));
    return LockCatalog(locks: locks);
  }

  static Future<LockCatalog> loadBundled() async =>
      fromJsonString(await rootBundle.loadString('assets/lock_catalog.json'));
}
