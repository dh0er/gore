/// One editor surface's pending contribution to the next save.
class PendingSaveEdit {
  const PendingSaveEdit({
    required this.edits,
    this.syncPersistentDataList = false,
    this.placementNotes = const [],
    this.clearPlacementNotes = const [],
    this.displayCount,
  }) : assert(displayCount == null || displayCount > 0),
       assert(displayCount == null || edits.length == 1);

  /// Raw write_save edit objects ({'path': ..., 'value': ...}).
  final List<Map<String, Object?>> edits;
  final bool syncPersistentDataList;

  /// Undo notes for NPC placement edits, recorded by the core AFTER the bytes
  /// land. Carried beside [edits] rather than as one of them because a note is
  /// not a change to the save: it goes to a sidecar file, and a failure to write
  /// it must not fail an otherwise good save.
  final List<Map<String, Object?>> placementNotes;

  /// NPC GlobalIds whose placement note is spent — sent by a restore, which puts
  /// the NPC back and so leaves nothing to undo.
  final List<String> clearPlacementNotes;

  /// Optional number shown by the global pending counter when one atomic core
  /// edit aggregates several user-visible changes (for example story flags).
  /// Persistence and partial-commit tracking still operate on [edits].
  final int? displayCount;

  int get pendingCount => displayCount ?? edits.length;
}
