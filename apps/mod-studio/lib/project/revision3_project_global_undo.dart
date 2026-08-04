import 'package:flutter/material.dart';

import '../core/mod_ffi.dart';
import 'revision3_project_history.dart';

const int _maxRevision3ProjectGlobalUndoRevision = 0x7fffffffffffffff;
final RegExp _revision3ProjectGlobalUndoProjectId = RegExp(r'^[0-9a-f]{32}$');

/// Exact visible managed-project authority captured before global Undo starts.
///
/// Hosts must return `null` from their checkpoint reader while a text edit is
/// dirty, recovery/reopen is required, or another project-wide action is busy.
/// The coordinator repeats that reader after every non-mutating await.
@immutable
final class Revision3ProjectGlobalUndoCheckpoint {
  const Revision3ProjectGlobalUndoCheckpoint({
    required this.root,
    required this.projectId,
    required this.projectRevision,
    required this.head,
  });

  final String root;
  final String projectId;
  final int projectRevision;
  final AuthoringWorkingHead head;

  bool sameAs(Revision3ProjectGlobalUndoCheckpoint other) =>
      root == other.root &&
      projectId == other.projectId &&
      projectRevision == other.projectRevision &&
      head.canonicalJson == other.head.canonicalJson;
}

/// One confirmed append-only Undo proposal.
///
/// [target] is always the immediate predecessor authenticated by a freshly
/// loaded [history]. Restoring it creates [nextRevision]; the fixed head never
/// moves backwards.
@immutable
final class Revision3ProjectGlobalUndoPlan {
  const Revision3ProjectGlobalUndoPlan._({
    required this.basis,
    required this.history,
    required this.target,
  });

  final Revision3ProjectGlobalUndoCheckpoint basis;
  final Revision3ProjectHistorySnapshot history;
  final Revision3ProjectHistoryEntry target;

  int get nextRevision => basis.projectRevision + 1;
}

enum Revision3ProjectGlobalUndoOutcome {
  restored,
  cancelled,
  nothingToUndo,
  unavailable,
  stale,
  superseded,
  busy,
}

@immutable
final class Revision3ProjectGlobalUndoResult {
  const Revision3ProjectGlobalUndoResult._(this.outcome, {this.publication});

  const Revision3ProjectGlobalUndoResult.restored(
    Revision3ProjectHistoryRestorePublication publication,
  ) : this._(
        Revision3ProjectGlobalUndoOutcome.restored,
        publication: publication,
      );

  const Revision3ProjectGlobalUndoResult.cancelled()
    : this._(Revision3ProjectGlobalUndoOutcome.cancelled);

  const Revision3ProjectGlobalUndoResult.nothingToUndo()
    : this._(Revision3ProjectGlobalUndoOutcome.nothingToUndo);

  const Revision3ProjectGlobalUndoResult.unavailable()
    : this._(Revision3ProjectGlobalUndoOutcome.unavailable);

  const Revision3ProjectGlobalUndoResult.stale()
    : this._(Revision3ProjectGlobalUndoOutcome.stale);

  const Revision3ProjectGlobalUndoResult.superseded()
    : this._(Revision3ProjectGlobalUndoOutcome.superseded);

  const Revision3ProjectGlobalUndoResult.busy()
    : this._(Revision3ProjectGlobalUndoOutcome.busy);

  final Revision3ProjectGlobalUndoOutcome outcome;
  final Revision3ProjectHistoryRestorePublication? publication;
}

final class Revision3ProjectGlobalUndoPublicationMismatch implements Exception {
  const Revision3ProjectGlobalUndoPublicationMismatch();

  @override
  String toString() =>
      'global Undo returned a publication outside its exact requested lineage';
}

typedef Revision3ProjectGlobalUndoCheckpointReader =
    Revision3ProjectGlobalUndoCheckpoint? Function();
typedef Revision3ProjectGlobalUndoHistoryLoader =
    Future<Revision3ProjectHistorySnapshot> Function(
      Revision3ProjectGlobalUndoCheckpoint basis,
    );
typedef Revision3ProjectGlobalUndoConfirmer =
    Future<bool> Function(Revision3ProjectGlobalUndoPlan plan);
typedef Revision3ProjectGlobalUndoRestorer =
    Future<Revision3ProjectHistoryRestorePublication> Function(
      Revision3ProjectGlobalUndoCheckpoint basis,
      Revision3ProjectHistorySnapshot expectedHistory,
      Revision3ProjectHistoryEntry target,
    );

/// Single-flight orchestration for the project-wide Undo command.
///
/// Loading and confirmation grant no mutation authority. Immediately before
/// [restore], the exact visible checkpoint must still equal the captured basis.
/// The restore callback remains the owning session/controller boundary and must
/// independently enforce its fixed-head CAS contract.
final class Revision3ProjectGlobalUndoCoordinator {
  factory Revision3ProjectGlobalUndoCoordinator({
    required Revision3ProjectGlobalUndoCheckpointReader readCurrentCheckpoint,
    required Revision3ProjectGlobalUndoHistoryLoader loadHistory,
    required Revision3ProjectGlobalUndoConfirmer confirm,
    required Revision3ProjectGlobalUndoRestorer restore,
  }) => Revision3ProjectGlobalUndoCoordinator._(
    readCurrentCheckpoint,
    loadHistory,
    confirm,
    restore,
  );

  Revision3ProjectGlobalUndoCoordinator._(
    this._readCurrentCheckpoint,
    this._loadHistory,
    this._confirm,
    this._restore,
  );

  final Revision3ProjectGlobalUndoCheckpointReader _readCurrentCheckpoint;
  final Revision3ProjectGlobalUndoHistoryLoader _loadHistory;
  final Revision3ProjectGlobalUndoConfirmer _confirm;
  final Revision3ProjectGlobalUndoRestorer _restore;

  bool _busy = false;
  bool _disposed = false;
  int _epoch = 0;

  bool get isBusy => _busy;

  /// Invalidates non-mutating late results and refuses every later invocation.
  ///
  /// A restore already entered cannot be cancelled safely. Its strict callback
  /// still completes, but a successful late result is reported as superseded
  /// rather than author-facing success.
  void dispose() {
    if (_disposed) return;
    _disposed = true;
    _epoch++;
  }

  Future<Revision3ProjectGlobalUndoResult> undo() async {
    if (_disposed) {
      return const Revision3ProjectGlobalUndoResult.unavailable();
    }
    if (_busy) return const Revision3ProjectGlobalUndoResult.busy();

    _busy = true;
    final epoch = ++_epoch;
    try {
      final basis = _readCurrentCheckpoint();
      if (!_validBasis(basis)) {
        return const Revision3ProjectGlobalUndoResult.unavailable();
      }

      final history = await _loadHistory(basis!);
      if (!_isCurrentEpoch(epoch)) {
        return const Revision3ProjectGlobalUndoResult.superseded();
      }
      if (!_stillCurrent(basis) || !_historyMatches(history, basis)) {
        return const Revision3ProjectGlobalUndoResult.stale();
      }

      final target = history.immediatePrevious;
      if (target == null) {
        return const Revision3ProjectGlobalUndoResult.nothingToUndo();
      }
      final plan = Revision3ProjectGlobalUndoPlan._(
        basis: basis,
        history: history,
        target: target,
      );
      final confirmed = await _confirm(plan);
      if (!_isCurrentEpoch(epoch)) {
        return const Revision3ProjectGlobalUndoResult.superseded();
      }
      if (!confirmed) {
        return const Revision3ProjectGlobalUndoResult.cancelled();
      }
      if (!_stillCurrent(basis)) {
        return const Revision3ProjectGlobalUndoResult.stale();
      }

      final publication = await _restore(basis, history, target);
      if (!_publicationMatches(publication, plan)) {
        throw const Revision3ProjectGlobalUndoPublicationMismatch();
      }
      if (!_isCurrentEpoch(epoch)) {
        return const Revision3ProjectGlobalUndoResult.superseded();
      }
      final published = _readCurrentCheckpoint();
      if (published == null ||
          published.root != basis.root ||
          published.projectId != basis.projectId ||
          published.projectRevision != plan.nextRevision ||
          published.head.canonicalJson != publication.head.canonicalJson) {
        throw const Revision3ProjectGlobalUndoPublicationMismatch();
      }
      return Revision3ProjectGlobalUndoResult.restored(publication);
    } finally {
      _busy = false;
    }
  }

  bool _isCurrentEpoch(int epoch) => !_disposed && epoch == _epoch;

  bool _stillCurrent(Revision3ProjectGlobalUndoCheckpoint basis) {
    final current = _readCurrentCheckpoint();
    return current != null && current.sameAs(basis);
  }
}

bool _validBasis(Revision3ProjectGlobalUndoCheckpoint? basis) =>
    basis != null &&
    basis.root.isNotEmpty &&
    _revision3ProjectGlobalUndoProjectId.hasMatch(basis.projectId) &&
    basis.projectId != '00000000000000000000000000000000' &&
    basis.projectRevision >= 0 &&
    basis.projectRevision < _maxRevision3ProjectGlobalUndoRevision;

bool _historyMatches(
  Revision3ProjectHistorySnapshot history,
  Revision3ProjectGlobalUndoCheckpoint basis,
) =>
    history.projectId == basis.projectId &&
    history.currentRevision == basis.projectRevision &&
    history.basisHead.canonicalJson == basis.head.canonicalJson &&
    history.current.head.canonicalJson == basis.head.canonicalJson;

bool _publicationMatches(
  Revision3ProjectHistoryRestorePublication publication,
  Revision3ProjectGlobalUndoPlan plan,
) =>
    publication.projectId == plan.basis.projectId &&
    publication.previousProjectRevision == plan.basis.projectRevision &&
    publication.projectRevision == plan.nextRevision &&
    publication.previousHead.canonicalJson == plan.basis.head.canonicalJson &&
    publication.restoredFromRevision == plan.target.projectRevision &&
    publication.restoredFromHead.canonicalJson ==
        plan.target.head.canonicalJson &&
    publication.head.canonicalJson != plan.basis.head.canonicalJson &&
    publication.head.canonicalJson != plan.target.head.canonicalJson;

/// Injected author-facing strings for the reusable confirmation dialog.
@immutable
final class Revision3ProjectGlobalUndoCopy {
  const Revision3ProjectGlobalUndoCopy({
    required this.title,
    required this.body,
    required this.projectOnlyBoundary,
    required this.cancel,
    required this.undo,
  });

  final String title;
  final String Function(int previousRevision, int nextRevision) body;
  final String projectOnlyBoundary;
  final String cancel;
  final String undo;
}

/// Shows only friendly revision numbers; heads, IDs, paths and seals stay out
/// of normal presentation.
Future<bool> showRevision3ProjectGlobalUndoConfirmation({
  required BuildContext context,
  required Revision3ProjectGlobalUndoPlan plan,
  required Revision3ProjectGlobalUndoCopy copy,
}) async {
  final confirmed = await showDialog<bool>(
    context: context,
    builder: (context) => AlertDialog(
      key: const Key('revision3-project-global-undo-dialog'),
      title: Text(copy.title),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(copy.body(plan.target.projectRevision, plan.nextRevision)),
          const SizedBox(height: 12),
          Text(copy.projectOnlyBoundary),
        ],
      ),
      actions: [
        TextButton(
          key: const Key('revision3-project-global-undo-cancel'),
          onPressed: () => Navigator.pop(context, false),
          child: Text(copy.cancel),
        ),
        FilledButton.icon(
          key: const Key('revision3-project-global-undo-confirm'),
          onPressed: () => Navigator.pop(context, true),
          icon: const Icon(Icons.undo),
          label: Text(copy.undo),
        ),
      ],
    ),
  );
  return confirmed == true;
}
