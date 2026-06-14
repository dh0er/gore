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
