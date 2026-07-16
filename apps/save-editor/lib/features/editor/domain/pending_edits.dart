/// One editor surface's pending contribution to the next save.
class PendingSaveEdit {
  const PendingSaveEdit({
    required this.edits,
    this.syncPersistentDataList = false,
    this.displayCount,
  }) : assert(displayCount == null || displayCount > 0),
       assert(displayCount == null || edits.length == 1);

  /// Raw write_save edit objects ({'path': ..., 'value': ...}).
  final List<Map<String, Object?>> edits;
  final bool syncPersistentDataList;

  /// Optional number shown by the global pending counter when one atomic core
  /// edit aggregates several user-visible changes (for example story flags).
  /// Persistence and partial-commit tracking still operate on [edits].
  final int? displayCount;

  int get pendingCount => displayCount ?? edits.length;
}
