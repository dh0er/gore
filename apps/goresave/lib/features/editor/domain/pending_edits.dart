/// One editor surface's pending contribution to the next save.
class PendingSaveEdit {
  const PendingSaveEdit({
    required this.edits,
    this.syncPersistentDataList = false,
  });

  /// Raw write_save edit objects ({'path': ..., 'value': ...}).
  final List<Map<String, Object?>> edits;
  final bool syncPersistentDataList;
}

/// A pending (unsaved) difficulty edit. Unlike [PendingSaveEdit], difficulty is
/// not a `write_save` edit object — it is its own multi-target
/// `write_difficulty` command (current save, optionally the profile's
/// `PersistentDataList.sav`, optionally every save of the profile). It is held
/// as a typed field on `EditorState` and saved as part of the global
/// `saveAllPending` flow.
///
/// The `difficulty` map is exactly the payload the core's `write_difficulty`
/// command expects (preset + optional custom sub-levels + flowHelper +
/// permadeath). The propagation flags drive which targets the save assembles.
class PendingDifficulty {
  const PendingDifficulty({
    required this.difficulty,
    this.alsoProfile = false,
    this.allSaves = false,
  });

  final Map<String, Object?> difficulty;

  /// Also write the resolved profile's `PersistentDataList.sav`.
  final bool alsoProfile;

  /// Apply to every save attributed to the resolved profile (current save
  /// always included).
  final bool allSaves;
}
