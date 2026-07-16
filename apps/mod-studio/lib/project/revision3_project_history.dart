import 'package:flutter/foundation.dart';

import '../core/mod_ffi.dart';

const int revision3ProjectHistoryMaxEntries = 256;

/// One exact, authenticated checkpoint reachable from the currently published
/// managed revision-3 head.
///
/// [head] is an opaque restore token. Author-facing surfaces must not render
/// its canonical bytes or content seal.
@immutable
final class Revision3ProjectHistoryEntry {
  const Revision3ProjectHistoryEntry({
    required this.head,
    required this.projectId,
    required this.projectRevision,
    required this.isCurrent,
  });

  final AuthoringWorkingHead head;
  final String projectId;
  final int projectRevision;
  final bool isCurrent;
}

/// Bounded newest-first history sealed by the exact current [basisHead].
///
/// Physical Store objects that are not retained by this manifest are never
/// represented here. [historyTruncated] truthfully records that older versions
/// expired at the retention boundary; there is no hidden cursor or CAS scan.
@immutable
final class Revision3ProjectHistorySnapshot {
  Revision3ProjectHistorySnapshot({
    required this.basisHead,
    required this.projectId,
    required this.currentRevision,
    required List<Revision3ProjectHistoryEntry> entries,
    required this.historyTruncated,
  }) : entries = List<Revision3ProjectHistoryEntry>.unmodifiable(entries) {
    if (currentRevision < 0 ||
        entries.isEmpty ||
        entries.length > revision3ProjectHistoryMaxEntries ||
        !entries.first.isCurrent ||
        entries.first.head.canonicalJson != basisHead.canonicalJson ||
        entries.first.projectId != projectId ||
        entries.first.projectRevision != currentRevision) {
      throw ArgumentError.value(
        entries,
        'entries',
        'must start with the exact current checkpoint',
      );
    }
    for (var index = 0; index < entries.length; index++) {
      final entry = entries[index];
      if (entry.projectId != projectId ||
          entry.isCurrent != (index == 0) ||
          entry.projectRevision != currentRevision - index) {
        throw ArgumentError.value(
          entries,
          'entries',
          'must be one contiguous newest-first project lineage',
        );
      }
    }
  }

  final AuthoringWorkingHead basisHead;
  final String projectId;
  final int currentRevision;
  final List<Revision3ProjectHistoryEntry> entries;
  final bool historyTruncated;

  Revision3ProjectHistoryEntry get current => entries.first;

  /// Null when the current checkpoint is the only recorded generation.
  Revision3ProjectHistoryEntry? get immediatePrevious =>
      entries.length < 2 ? null : entries[1];

  int get earliestVisibleRevision => entries.last.projectRevision;
}

/// Durable receipt for one append-only history restore.
///
/// The fixed head always advances to [head]; it is never moved back to
/// [restoredFromHead].
@immutable
final class Revision3ProjectHistoryRestorePublication {
  const Revision3ProjectHistoryRestorePublication({
    required this.previousHead,
    required this.head,
    required this.projectId,
    required this.previousProjectRevision,
    required this.projectRevision,
    required this.restoredFromHead,
    required this.restoredFromRevision,
  });

  final AuthoringWorkingHead previousHead;
  final AuthoringWorkingHead head;
  final String projectId;
  final int previousProjectRevision;
  final int projectRevision;
  final AuthoringWorkingHead restoredFromHead;
  final int restoredFromRevision;
}

typedef Revision3ProjectHistoryLoader =
    Future<Revision3ProjectHistorySnapshot> Function();

typedef Revision3ProjectHistoryRestorer =
    Future<Revision3ProjectHistoryRestorePublication> Function(
      Revision3ProjectHistorySnapshot expectedHistory,
      Revision3ProjectHistoryEntry target,
    );
